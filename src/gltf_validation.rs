use gltf::accessor::{DataType, Dimensions};
use gltf::buffer::{Data, View};
use gltf::{Accessor, Primitive, Semantic};

/// Checks the accessor preconditions required by the glTF geometry readers.
pub(super) fn validate_primitive(primitive: &Primitive<'_>, buffers: &[Data]) -> Result<(), String> {
    let positions = primitive
        .get(&Semantic::Positions)
        .ok_or_else(|| "Missing glTF vertex positions".to_string())?;
    if positions.data_type() != DataType::F32
        || positions.dimensions() != Dimensions::Vec3
        || positions.normalized()
    {
        return Err("glTF positions must be nonnormalized FLOAT VEC3 values".to_string());
    }
    validate_accessor(&positions, buffers)?;

    if let Some(indices) = primitive.indices() {
        if !matches!(
            indices.data_type(),
            DataType::U8 | DataType::U16 | DataType::U32
        ) || indices.dimensions() != Dimensions::Scalar
            || indices.normalized()
        {
            return Err("glTF indices must be nonnormalized unsigned SCALAR values".to_string());
        }
        validate_accessor(&indices, buffers)?;
    }
    Ok(())
}

fn validate_accessor(accessor: &Accessor<'_>, buffers: &[Data]) -> Result<(), String> {
    let count = accessor.count();
    // SparseIter uses a u32 counter and increments it after every element.
    if count == 0 || u32::try_from(count).is_err() {
        return Err(format!(
            "Invalid count for glTF accessor {}",
            accessor.index()
        ));
    }
    let size = accessor.size();
    if let Some(view) = accessor.view() {
        checked_range(view, accessor.offset(), count, size, buffers)?;
    }

    if let Some(sparse) = accessor.sparse() {
        if sparse.count() == 0 || sparse.count() > count {
            return Err(format!(
                "Invalid sparse count for glTF accessor {}",
                accessor.index()
            ));
        }
        let indices = sparse.indices();
        let index_size = indices.index_type().size();
        let (index_data, stride) = checked_range(
            indices.view(),
            indices.offset(),
            sparse.count(),
            index_size,
            buffers,
        )?;
        let values = sparse.values();
        checked_range(
            values.view(),
            values.offset(),
            sparse.count(),
            size,
            buffers,
        )?;

        let mut previous = None;
        for entry in index_data.chunks(stride) {
            let bytes = entry
                .get(..index_size)
                .ok_or_else(|| "Truncated glTF sparse index".to_string())?;
            let index = match bytes {
                [a] => u32::from(*a),
                [a, b] => u32::from(u16::from_le_bytes([*a, *b])),
                [a, b, c, d] => u32::from_le_bytes([*a, *b, *c, *d]),
                _ => return Err("Invalid glTF sparse index size".to_string()),
            };
            let index = usize::try_from(index)
                .map_err(|_| "glTF sparse index exceeds platform limits".to_string())?;
            if index >= count || matches!(previous, Some(last) if index <= last) {
                return Err(format!(
                    "glTF accessor {} has out-of-range or unordered sparse indices",
                    accessor.index()
                ));
            }
            previous = Some(index);
        }
    }
    Ok(())
}

/// Checks a reader's byte range and returns its bytes and element stride.
///
/// - `view`: Buffer view containing the elements.
/// - `offset`: Byte offset of the first element within the view.
/// - `count`: Number of elements to read; must be positive.
/// - `size`: Number of bytes occupied by one element.
/// - `buffers`: Imported buffer data, in document order.
/// - Returns the checked byte range and stride, or an invalid-layout error.
fn checked_range<'a>(
    view: View<'_>,
    offset: usize,
    count: usize,
    size: usize,
    buffers: &'a [Data],
) -> Result<(&'a [u8], usize), String> {
    let invalid = || format!("Invalid byte range for glTF buffer view {}", view.index());
    let stride = view.stride().unwrap_or(size);
    if size == 0 || stride < size {
        return Err(invalid());
    }
    let end = count
        .checked_sub(1)
        .and_then(|last| stride.checked_mul(last))
        .and_then(|length| offset.checked_add(length))
        .and_then(|last| last.checked_add(size))
        .ok_or_else(invalid)?;
    let view_end = view.offset().checked_add(view.length()).ok_or_else(invalid)?;
    let buffer = view.buffer();
    if view_end > buffer.length() {
        return Err(invalid());
    }
    let data = buffers.get(buffer.index()).ok_or_else(invalid)?;
    let view_data = data.get(view.offset()..view_end).ok_or_else(invalid)?;
    let range = view_data.get(offset..end).ok_or_else(invalid)?;
    Ok((range, stride))
}
