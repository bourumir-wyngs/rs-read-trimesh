//! Bounded, non-blocking document reads shared by every loader.
//!
//! A loader must never block forever on a FIFO or a hung filesystem, and must
//! never allocate an unbounded buffer for a device such as `/dev/zero`. Every
//! format therefore reads its document through [`read_document`], which only
//! accepts regular files and enforces a byte limit and a deadline.
use std::io::Read;
use std::path::Path;
use std::sync::{Arc, Condvar, Mutex, OnceLock, mpsc};
use std::time::{Duration, Instant};

/// The size limit for formats that do not define a smaller one of their own.
pub(crate) const MAX_DOCUMENT_BYTES: usize = 256 * 1024 * 1024;

/// The time budget for a single open-and-read, or for a group of related reads.
pub(crate) const TIMEOUT: Duration = Duration::from_secs(5);

struct Workers {
    active: Mutex<usize>,
    available: Condvar,
    limit: usize,
}

impl Workers {
    fn new(limit: usize) -> Arc<Self> {
        Arc::new(Self {
            active: Mutex::new(0),
            available: Condvar::new(),
            limit,
        })
    }

    fn reserve(self: &Arc<Self>, deadline: Instant) -> Result<WorkerSlot, String> {
        let mut active = self.active.lock().unwrap_or_else(|e| e.into_inner());
        loop {
            let remaining = time_left(deadline)?;
            if *active < self.limit {
                *active += 1;
                return Ok(WorkerSlot(Arc::clone(self)));
            }
            (active, _) = self
                .available
                .wait_timeout(active, remaining)
                .unwrap_or_else(|e| e.into_inner());
        }
    }
}

struct WorkerSlot(Arc<Workers>);

impl Drop for WorkerSlot {
    fn drop(&mut self) {
        *self.0.active.lock().unwrap_or_else(|e| e.into_inner()) -= 1;
        self.0.available.notify_one();
    }
}

fn workers() -> &'static Arc<Workers> {
    static WORKERS: OnceLock<Arc<Workers>> = OnceLock::new();
    WORKERS.get_or_init(|| Workers::new(16))
}

pub(crate) fn time_left(deadline: Instant) -> Result<Duration, String> {
    deadline
        .checked_duration_since(Instant::now())
        .filter(|remaining| !remaining.is_zero())
        .ok_or_else(|| "File read timed out".to_owned())
}

/// Runs one filesystem operation on a bounded pool, honouring the deadline.
pub(crate) fn run_with_deadline<T: Send + 'static>(
    deadline: Instant,
    work: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    run_on(workers(), deadline, work)
}

fn run_on<T: Send + 'static>(
    workers: &Arc<Workers>,
    deadline: Instant,
    work: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    let slot = workers.reserve(deadline)?;
    let (send, receive) = mpsc::sync_channel(1);
    // Rust cannot cancel a thread stuck in a filesystem/kernel call. Keep its
    // slot until it actually exits, so repeated timeouts cannot leak an
    // unlimited number of threads, open files, or read buffers.
    std::thread::Builder::new()
        .name("mesh-document".to_owned())
        .spawn(move || {
            let result = time_left(deadline).and_then(|_| work());
            let _ = send.send(result);
            drop(slot);
        })
        .map_err(|e| format!("Cannot start file reader: {e}"))?;
    receive
        .recv_timeout(time_left(deadline)?)
        .map_err(|e| match e {
            mpsc::RecvTimeoutError::Timeout => "File read timed out".to_owned(),
            mpsc::RecvTimeoutError::Disconnected => "File reader stopped".to_owned(),
        })?
}

/// Opens without waiting for a FIFO peer or a device to become ready.
pub(crate) fn open_options() -> std::fs::OpenOptions {
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NONBLOCK);
    }
    options
}

/// Reads a document without trusting a named pipe, device, or file size.
pub(crate) fn read_document(path: &Path, limit: usize) -> Result<Vec<u8>, String> {
    let path = path.to_owned();
    let deadline = Instant::now() + TIMEOUT;
    run_with_deadline(deadline, move || {
        let file = open_options().open(path).map_err(|e| e.to_string())?;
        read_regular_file(file, limit, deadline)
    })
}

pub(crate) fn read_regular_file(
    mut file: std::fs::File,
    limit: usize,
    deadline: Instant,
) -> Result<Vec<u8>, String> {
    // Inspect the opened handle so a path replacement cannot swap a regular
    // file for a FIFO/device between inspection and reading.
    let metadata = file.metadata().map_err(|e| e.to_string())?;
    if !metadata.is_file() {
        return Err("Mesh documents and resources must be regular files".to_owned());
    }
    if metadata.len() > limit as u64 {
        return Err(format!("File exceeds the {limit}-byte limit"));
    }
    time_left(deadline)?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(metadata.len() as usize)
        .map_err(|e| format!("Cannot allocate file buffer: {e}"))?;
    let mut chunk = [0; 64 * 1024];
    loop {
        time_left(deadline)?;
        // Read one byte beyond the limit to detect files which grow after the
        // metadata check, without allocating an unbounded read_to_end buffer.
        let requested = chunk.len().min(limit - bytes.len() + 1);
        let read = match file.read(&mut chunk[..requested]) {
            Ok(read) => read,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error.to_string()),
        };
        if read == 0 {
            return Ok(bytes);
        }
        if read > limit - bytes.len() {
            return Err(format!("File exceeds the {limit}-byte limit"));
        }
        if bytes.capacity() - bytes.len() < read {
            let capacity = bytes
                .capacity()
                .saturating_mul(2)
                .max(bytes.len() + read)
                .min(limit);
            bytes
                .try_reserve_exact(capacity - bytes.len())
                .map_err(|e| format!("Cannot grow file buffer: {e}"))?;
        }
        bytes.extend_from_slice(&chunk[..read]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeout_keeps_worker_slot_until_work_finishes() {
        let workers = Workers::new(1);
        let (release, blocked) = mpsc::channel();
        let (started, began) = mpsc::channel();
        let (finished, done) = mpsc::channel();
        let runner = Arc::clone(&workers);
        let caller = std::thread::spawn(move || {
            run_on(
                &runner,
                Instant::now() + Duration::from_millis(200),
                move || {
                    started.send(()).unwrap();
                    // Bounded even if the test assertion fails before releasing it.
                    let _ = blocked.recv_timeout(Duration::from_secs(2));
                    finished.send(()).unwrap();
                    Ok(())
                },
            )
        });
        began.recv_timeout(Duration::from_secs(1)).unwrap();
        assert!(caller.join().unwrap().unwrap_err().contains("timed out"));
        assert_eq!(*workers.active.lock().unwrap(), 1);
        let error =
            run_on::<()>(&workers, Instant::now() + Duration::from_millis(30), || {
                panic!("saturated workers must not run additional work")
            })
            .unwrap_err();
        assert!(error.contains("timed out"));
        release.send(()).unwrap();
        done.recv_timeout(Duration::from_secs(1)).unwrap();
        // Reserving the next slot also waits for the old worker to drop it.
        assert_eq!(
            run_on(&workers, Instant::now() + Duration::from_secs(1), || Ok(42))
                .unwrap(),
            42
        );
    }

    #[cfg(unix)]
    #[test]
    fn device_documents_are_rejected() {
        for device in ["/dev/null", "/dev/zero"] {
            let error = read_document(Path::new(device), MAX_DOCUMENT_BYTES).unwrap_err();
            assert!(error.contains("regular files"), "{error}");
        }
    }
}
