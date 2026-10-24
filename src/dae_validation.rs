//! Bound XML work and check arithmetic before dae-parser builds its document.
use crate::file_read;
use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};
use std::path::Path;

const MAX_DOCUMENT_BYTES: usize = 64 * 1024 * 1024;
const MAX_DEPTH: usize = 128;
const MAX_ATTRIBUTES: usize = 128;

/// COLLADA keeps a tighter limit than other formats because its parser
/// allocates per declared count, bounded by the document size.
pub(super) fn read_document(path: &Path) -> Result<Vec<u8>, String> {
    file_read::read_document(path, MAX_DOCUMENT_BYTES)
}

#[derive(Default)]
struct Element {
    // Only lines and triangles multiply their declared count by a tuple width.
    primitive: Option<(usize, usize)>,
    input_stride: usize,
    opaque: bool,
    fx_profile: bool,
}

impl Element {
    fn finish(self) -> Result<(), String> {
        if let Some((count, vertices_per_primitive)) = self.primitive {
            self.input_stride
                .checked_mul(vertices_per_primitive)
                .and_then(|width| width.checked_mul(count))
                .ok_or("COLLADA primitive count overflows the index range")?;
        }
        Ok(())
    }
}

pub(super) fn validate(bytes: &[u8]) -> Result<(), String> {
    let mut reader = Reader::from_reader(bytes);
    let mut stack = Vec::new();
    let mut array_values = 0usize;
    loop {
        let event = reader
            .read_event()
            .map_err(|e| format!("Invalid COLLADA XML: {e}"))?;
        let empty = matches!(event, Event::Empty(_));
        match event {
            Event::Start(start) | Event::Empty(start) => {
                if stack.len() >= MAX_DEPTH {
                    return Err("COLLADA XML exceeds the depth limit of 128".to_string());
                }
                let element = check_element(
                    &reader,
                    &start,
                    bytes.len(),
                    &mut array_values,
                    stack.last_mut(),
                )?;
                if empty {
                    element.finish()?;
                } else {
                    stack.push(element);
                }
            }
            Event::End(_) => {
                stack
                    .pop()
                    .ok_or("Unexpected COLLADA closing tag")?
                    .finish()?;
            }
            Event::Eof if stack.is_empty() => return Ok(()),
            Event::Eof => return Err("Unclosed COLLADA XML element".to_string()),
            _ => {}
        }
    }
}

fn check_element(
    reader: &Reader<&[u8]>,
    start: &BytesStart<'_>,
    document_len: usize,
    array_values: &mut usize,
    parent: Option<&mut Element>,
) -> Result<Element, String> {
    let name = start.local_name();
    let name = name.as_ref();
    // Core <technique> preserves arbitrary XML. FX profile techniques are
    // interpreted by dae-parser and must keep their numeric checks.
    let opaque = parent.as_ref().is_some_and(|parent| parent.opaque)
        || (name == b"technique" && !parent.as_ref().is_some_and(|parent| parent.fx_profile));
    let is_array = matches!(
        name,
        b"float_array"
            | b"int_array"
            | b"bool_array"
            | b"Name_array"
            | b"IDREF_array"
            | b"SIDREF_array"
            | b"token_array"
    );
    let has_count = is_array || matches!(name, b"triangles" | b"lines" | b"accessor");
    let mut count = 0;
    let mut offset = 0;
    let mut stride = 1;
    for (index, attribute) in start.attributes().enumerate() {
        if index >= MAX_ATTRIBUTES {
            return Err("COLLADA XML exceeds the limit of 128 attributes per element".to_string());
        }
        let attribute = attribute.map_err(|e| format!("Invalid COLLADA attribute: {e}"))?;
        if opaque {
            continue;
        }
        let number = match attribute.key.as_ref() {
            b"count" if has_count => &mut count,
            b"offset" if matches!(name, b"input" | b"accessor") => &mut offset,
            b"stride" if name == b"accessor" => &mut stride,
            _ => continue,
        };
        let decoded = reader
            .decoder()
            .decode(&attribute.value)
            .map_err(|e| format!("Invalid COLLADA attribute: {e}"))?;
        *number = quick_xml::escape::unescape(&decoded)
            .map_err(|e| format!("Invalid COLLADA attribute: {e}"))?
            .parse::<usize>()
            .map_err(|_| "Invalid COLLADA count, offset or stride")?;
    }
    if opaque {
        return Ok(Element {
            opaque: true,
            ..Element::default()
        });
    }
    if has_count && count > document_len {
        return Err("COLLADA count exceeds available document data".to_string());
    }
    if is_array {
        *array_values = array_values
            .checked_add(count)
            .filter(|&total| total <= document_len)
            .ok_or("COLLADA array counts exceed available document data")?;
    }
    if name == b"input" {
        // dae-parser computes this addition in u32 before validating primitives.
        let width = u32::try_from(offset)
            .ok()
            .and_then(|offset| offset.checked_add(1))
            .ok_or("COLLADA input offset exceeds the supported range")?
            as usize;
        if let Some(parent) = parent {
            parent.input_stride = parent.input_stride.max(width);
        }
    }
    if name == b"accessor" {
        stride
            .checked_mul(count)
            .and_then(|size| size.checked_add(offset))
            .ok_or("COLLADA accessor range overflows")?;
    }
    Ok(Element {
        fx_profile: matches!(
            name,
            b"profile_COMMON" | b"profile_CG" | b"profile_GLES" | b"profile_GLSL"
        ),
        primitive: match name {
            b"triangles" => Some((count, 3)),
            b"lines" => Some((count, 2)),
            _ => None,
        },
        ..Element::default()
    })
}
