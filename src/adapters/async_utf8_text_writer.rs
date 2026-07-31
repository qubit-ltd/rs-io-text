use std::ops::{Deref, DerefMut};

use qubit_codec_text::Utf8Codec;
use qubit_io::AsyncOutput;

use crate::{AsyncCharsetTextWriter, CodingErrorPolicy};

/// Convenience asynchronous UTF-8 writer.
#[derive(Debug)]
pub struct AsyncUtf8TextWriter<O>(AsyncCharsetTextWriter<O, Utf8Codec>)
where
    O: AsyncOutput<Item = u8>;

impl<O> AsyncUtf8TextWriter<O>
where
    O: AsyncOutput<Item = u8>,
{
    /// Creates a strict UTF-8 writer with the default capacity.
    #[must_use]
    pub fn new(output: O) -> Self {
        Self::with_policy(output, CodingErrorPolicy::Strict)
    }

    /// Creates a UTF-8 writer with an explicit error policy.
    #[must_use]
    pub fn with_policy(output: O, policy: CodingErrorPolicy) -> Self {
        Self(AsyncCharsetTextWriter::new(output, Utf8Codec, policy))
    }

    /// Creates a UTF-8 writer with an explicit byte capacity.
    #[must_use]
    pub fn with_capacity(output: O, policy: CodingErrorPolicy, capacity: usize) -> Self {
        Self(AsyncCharsetTextWriter::new_with_buffer_capacity(
            output, Utf8Codec, policy, capacity,
        ))
    }

    /// Consumes the adapter and returns its generic UTF-8 writer.
    #[must_use]
    pub fn into_inner(self) -> AsyncCharsetTextWriter<O, Utf8Codec> {
        self.0
    }
}

impl<O> Deref for AsyncUtf8TextWriter<O>
where
    O: AsyncOutput<Item = u8>,
{
    type Target = AsyncCharsetTextWriter<O, Utf8Codec>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<O> DerefMut for AsyncUtf8TextWriter<O>
where
    O: AsyncOutput<Item = u8>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
