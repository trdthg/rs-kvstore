use bytes::Bytes;

enum Frame<T>
where
    T: ToString,
{
    Simple(T),
    Error(T),
    Integer(u64),
    Bulk(Bytes),
    Null,
    Array(Vec<Frame<T>>),
}

// enum HttpFrame {
//     RequestHead {
//         method: Method,
//         url: Uri,
//         version: Version,
//         headers: HeaderMap,
//     },
//     ResponseHead {
//         status: StatusCode,
//         version: Version,
//         headers: HeaderMap,
//     },
//     BodyChunk {
//         chunk: Bytes,
//     },
// }
