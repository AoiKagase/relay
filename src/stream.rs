use crate::error::{Error, ErrorKind};
use actix_web::web::{Bytes, BytesMut};
use futures_core::Stream;
use streem::IntoStreamer;

pub(crate) fn limit_stream<'a, S>(
    input: S,
    limit: usize,
) -> impl Stream<Item = Result<Bytes, Error>> + Send + 'a
where
    S: Stream<Item = reqwest::Result<Bytes>> + Send + 'a,
{
    streem::try_from_fn(move |yielder| async move {
        let stream = std::pin::pin!(input);
        let mut stream = stream.into_streamer();

        let mut count = 0;

        while let Some(bytes) = stream.try_next().await? {
            count += bytes.len();

            if count > limit {
                return Err(ErrorKind::BodyTooLarge.into());
            }

            yielder.yield_ok(bytes).await;
        }

        Ok(())
    })
}

pub(crate) async fn aggregate<S>(input: S) -> Result<Bytes, Error>
where
    S: Stream<Item = Result<Bytes, Error>>,
{
    let stream = std::pin::pin!(input);
    let mut streamer = stream.into_streamer();

    let mut buf = Vec::new();

    while let Some(bytes) = streamer.try_next().await? {
        buf.push(bytes);
    }

    if buf.len() == 1 {
        return Ok(buf.pop().expect("buf has exactly one element"));
    }

    let total_size: usize = buf.iter().map(|b| b.len()).sum();

    let mut bytes_mut = BytesMut::with_capacity(total_size);

    for bytes in &buf {
        bytes_mut.extend_from_slice(&bytes);
    }

    Ok(bytes_mut.freeze())
}
