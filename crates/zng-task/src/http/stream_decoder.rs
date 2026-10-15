use std::{pin::Pin, task::Poll};

use futures_lite::AsyncRead;

const BUF_SIZE: usize = 1024;

pub struct StreamDecoder {
    input: Pin<Box<dyn AsyncRead + Send>>,
    decoder: encoding_rs::Decoder,

    // input_buf: [..BUF_SIZE], output_buf: [BUF_SIZE..]
    buf: Box<[u8; BUF_SIZE * 2]>,
    input_start: usize,
    input_end: usize,
    output_start: usize,
    output_end: usize,

    eof: bool,
    finished: bool,
}
impl StreamDecoder {
    pub fn new(input: Box<dyn AsyncRead + Send>, decoder: encoding_rs::Decoder) -> Self {
        Self {
            input: Box::into_pin(input),
            decoder,
            buf: Box::new([0; BUF_SIZE * 2]),
            input_start: 0,
            input_end: 0,
            output_start: 0,
            output_end: 0,
            eof: false,
            finished: false,
        }
    }
}
impl AsyncRead for StreamDecoder {
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>, buf: &mut [u8]) -> Poll<std::io::Result<usize>> {
        let this = self.as_mut().get_mut();

        if buf.is_empty() {
            return Poll::Ready(Ok(0));
        }

        loop {
            // return  previously decoded
            if this.output_start < this.output_end {
                let n = buf.len().min(this.output_end - this.output_start);

                buf[..n].copy_from_slice(&this.buf[BUF_SIZE..][this.output_start..this.output_start + n]);

                this.output_start += n;
                return Poll::Ready(Ok(n));
            }

            if this.finished {
                return Poll::Ready(Ok(0));
            }

            this.output_start = 0;
            this.output_end = 0;

            // refill input
            if this.input_start == this.input_end && !this.eof {
                this.input_start = 0;
                this.input_end = 0;

                match this.input.as_mut().poll_read(cx, &mut this.buf[..BUF_SIZE]) {
                    Poll::Pending => return Poll::Pending,
                    Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                    Poll::Ready(Ok(0)) => this.eof = true,
                    Poll::Ready(Ok(n)) => this.input_end = n,
                }
            }

            let (input_buf, output_buf) = this.buf.split_at_mut(BUF_SIZE);
            let (result, read, written, _) =
                this.decoder
                    .decode_to_utf8(&input_buf[this.input_start..this.input_end], output_buf, this.eof);

            this.input_start += read;
            this.output_end = written;

            if result == encoding_rs::CoderResult::InputEmpty && this.eof {
                this.finished = true;
            }
        }
    }
}
