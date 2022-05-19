use futures::io::{AsyncRead, AsyncWrite};
use minicbor::{Encode, Decode};
use minicbor_io::{AsyncReader, AsyncWriter, Error};
use std::fmt::Debug;

pub async fn send<T, W>(w: &mut AsyncWriter<W>, v: T) -> Result<usize, Error>
where
    T: Encode<()> + Debug,
    W: AsyncWrite + Unpin
{
    log::trace!("send: {:?}", v);
    w.write(v).await
}

pub async fn recv<'a, T, R>(r: &'a mut AsyncReader<R>) -> Result<Option<T>, Error>
where
    T: Decode<'a, ()> + Debug,
    R: AsyncRead + Unpin
{
    let v = r.read().await?;
    log::trace!("recv: {:?}", v);
    Ok(v)
}

