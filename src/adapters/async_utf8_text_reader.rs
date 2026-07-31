use std::ops::{
    Deref,
    DerefMut,
};

use qubit_codec_text::Utf8Codec;
use qubit_io::AsyncInput;

use crate::{
    AsyncCharsetTextReader,
    CodingErrorPolicy,
};

/// Convenience asynchronous UTF-8 reader.
#[derive(Debug)]
pub struct AsyncUtf8TextReader<I>(AsyncCharsetTextReader<I, Utf8Codec>)
where
    I: AsyncInput<Item = u8>;

impl<I> AsyncUtf8TextReader<I>
where
    I: AsyncInput<Item = u8>,
{
    /// Creates a strict UTF-8 reader with the default capacity.
    #[must_use]
    pub fn new(input: I) -> Self {
        Self::with_policy(input, CodingErrorPolicy::Strict)
    }

    /// Creates a UTF-8 reader with an explicit error policy.
    #[must_use]
    pub fn with_policy(input: I, policy: CodingErrorPolicy) -> Self {
        Self(AsyncCharsetTextReader::new(input, Utf8Codec, policy))
    }

    /// Creates a UTF-8 reader with an explicit byte capacity.
    #[must_use]
    pub fn with_capacity(
        input: I,
        policy: CodingErrorPolicy,
        capacity: usize,
    ) -> Self {
        Self(AsyncCharsetTextReader::new_with_buffer_capacity(
            input, Utf8Codec, policy, capacity,
        ))
    }

    /// Consumes the adapter and returns its generic UTF-8 reader.
    #[must_use]
    pub fn into_inner(self) -> AsyncCharsetTextReader<I, Utf8Codec> {
        self.0
    }
}

impl<I> Deref for AsyncUtf8TextReader<I>
where
    I: AsyncInput<Item = u8>,
{
    type Target = AsyncCharsetTextReader<I, Utf8Codec>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<I> DerefMut for AsyncUtf8TextReader<I>
where
    I: AsyncInput<Item = u8>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
