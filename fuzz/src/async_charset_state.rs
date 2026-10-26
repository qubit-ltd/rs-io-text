// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::error::Error;
use std::fmt;
use std::io;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

use qubit_codec_text::CharsetEncodePolicy;
use qubit_codec_text::Utf8Codec;
use qubit_io::AsyncInput;
use qubit_io::AsyncOutput;
use qubit_io_text::AsyncCharsetTextReader;
use qubit_io_text::AsyncCharsetTextWriter;
use qubit_io_text::LineEndingSet;

const INPUT_LIMIT: usize = 4096;
const OP_LIMIT: usize = 64;
const EVENT_LIMIT: usize = 64;
const POLL_LIMIT: usize = 65_536;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Event {
    Ready(usize),
    Pending,
    Error,
}

fn decode_event(byte: u8) -> Event {
    match byte & 3 {
        1 => Event::Pending,
        2 => Event::Error,
        3 => Event::Ready(8),
        _ => Event::Ready(usize::from(byte >> 2) % 8 + 1),
    }
}

#[derive(Clone, Copy, Default)]
struct Stats {
    pending: usize,
    read_errors: usize,
    write_errors: usize,
    flush_errors: usize,
    cancelled: usize,
    finish_retries: usize,
    rejected_records: usize,
    polls: usize,
    operations: usize,
    events: usize,
    write_str_ops: usize,
    write_char_ops: usize,
    flush_ops: usize,
    read_char_ops: usize,
    read_batch_ops: usize,
    line_ops: usize,
    reader_eof: bool,
    finish_entries: usize,
    eof_tail_records: usize,
}

fn assert_text_read_result(expected: &str, actual: &str, reached_eof: bool) {
    assert!(expected.starts_with(actual), "reader text is not a source prefix");
    if reached_eof {
        assert_eq!(actual, expected, "EOF text incomplete");
    }
}

fn assert_record_read_result(expected: &[String], actual: &[String], reached_eof: bool) {
    assert!(expected.starts_with(actual), "reader records are not a source prefix");
    if reached_eof {
        assert_eq!(actual, expected, "EOF records incomplete");
    }
}

fn assert_rejected_record_count(expected: usize, actual: usize, reached_eof: bool) {
    assert!(actual <= expected, "too many rejected records");
    if reached_eof {
        assert_eq!(actual, expected, "EOF rejected records incomplete");
    }
}

#[derive(Clone, Copy, Debug)]
enum FaultKind {
    Read,
    Write,
    Flush,
}

#[derive(Debug)]
struct InjectedFault {
    kind: FaultKind,
    event: usize,
}

impl fmt::Display for InjectedFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "injected {:?} fault at event {}", self.kind, self.event)
    }
}

impl Error for InjectedFault {}

#[derive(Default)]
struct Schedule {
    events: Vec<Event>,
    cursor: usize,
}

impl Schedule {
    fn new(events: Vec<Event>) -> Self {
        Self { events, cursor: 0 }
    }

    fn next(&mut self) -> (Event, usize) {
        let index = self.cursor;
        let event = self.events.get(index).copied().unwrap_or(Event::Ready(8));
        self.cursor = self.cursor.saturating_add(1);
        (event, index)
    }
}

struct ScriptInput {
    bytes: Vec<u8>,
    position: usize,
    schedule: Schedule,
    stats: Stats,
}

impl ScriptInput {
    fn new(bytes: Vec<u8>, events: Vec<Event>) -> Self {
        Self {
            bytes,
            position: 0,
            schedule: Schedule::new(events),
            stats: Stats::default(),
        }
    }
}

impl AsyncInput for ScriptInput {
    type Item = u8;

    unsafe fn poll_read_unchecked(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        output: &mut [u8],
        index: usize,
        count: usize,
    ) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        assert!(index <= output.len() && count <= output.len() - index);
        if count == 0 {
            return Poll::Ready(Ok(0));
        }
        let (event, event_index) = this.schedule.next();
        match event {
            Event::Pending => {
                this.stats.pending += 1;
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Event::Error => {
                this.stats.read_errors += 1;
                Poll::Ready(Err(io::Error::new(
                    io::ErrorKind::Other,
                    InjectedFault {
                        kind: FaultKind::Read,
                        event: event_index,
                    },
                )))
            }
            Event::Ready(limit) => {
                let count = count.min(limit).min(this.bytes.len().saturating_sub(this.position));
                output[index..index + count].copy_from_slice(&this.bytes[this.position..this.position + count]);
                this.position += count;
                Poll::Ready(Ok(count))
            }
        }
    }
}

struct ScriptOutput {
    bytes: Vec<u8>,
    schedule: Schedule,
    flush_schedule: Schedule,
    stats: Stats,
}

impl ScriptOutput {
    fn new(write_events: Vec<Event>, flush_events: Vec<Event>) -> Self {
        Self {
            bytes: Vec::new(),
            schedule: Schedule::new(write_events),
            flush_schedule: Schedule::new(flush_events),
            stats: Stats::default(),
        }
    }
}

impl AsyncOutput for ScriptOutput {
    type Item = u8;

    unsafe fn poll_write_unchecked(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        input: &[u8],
        index: usize,
        count: usize,
    ) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        assert!(index <= input.len() && count <= input.len() - index);
        if count == 0 {
            return Poll::Ready(Ok(0));
        }
        let (event, event_index) = this.schedule.next();
        match event {
            Event::Pending => {
                this.stats.pending += 1;
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Event::Error => {
                this.stats.write_errors += 1;
                Poll::Ready(Err(io::Error::new(
                    io::ErrorKind::Other,
                    InjectedFault {
                        kind: FaultKind::Write,
                        event: event_index,
                    },
                )))
            }
            Event::Ready(limit) => {
                let count = count.min(limit);
                this.bytes.extend_from_slice(&input[index..index + count]);
                Poll::Ready(Ok(count))
            }
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        let (event, event_index) = this.flush_schedule.next();
        match event {
            Event::Pending => {
                this.stats.pending += 1;
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Event::Error => {
                this.stats.flush_errors += 1;
                Poll::Ready(Err(io::Error::new(
                    io::ErrorKind::Other,
                    InjectedFault {
                        kind: FaultKind::Flush,
                        event: event_index,
                    },
                )))
            }
            Event::Ready(_) => Poll::Ready(Ok(())),
        }
    }
}

#[derive(Clone, Copy)]
struct Header {
    scene: usize,
    capacity: usize,
    endings: usize,
    event_count: usize,
    cancel: u8,
}

fn parse(data: &[u8]) -> (Header, Vec<u8>, Vec<Event>, Vec<u8>) {
    let data = &data[..data.len().min(INPUT_LIMIT)];
    let mut header = [0_u8; 6];
    let copy = data.len().min(header.len());
    header[..copy].copy_from_slice(&data[..copy]);
    let op_count = usize::from(header[3]) % (OP_LIMIT + 1);
    let event_count = usize::from(header[4]) % (EVENT_LIMIT + 1);
    let op_end = 6 + op_count.min(data.len().saturating_sub(6));
    let events_end = op_end + event_count.min(data.len().saturating_sub(op_end));
    let operations = data.get(6..op_end).unwrap_or_default().to_vec();
    let events = data
        .get(op_end..events_end)
        .unwrap_or_default()
        .iter()
        .copied()
        .map(decode_event)
        .collect();
    let payload = data.get(events_end..).unwrap_or_default().to_vec();
    (
        Header {
            scene: usize::from(header[0] % 4),
            capacity: usize::from(header[1] % 8 + 1),
            endings: usize::from(header[2] % 4),
            event_count,
            cancel: header[5],
        },
        operations,
        events,
        payload,
    )
}

/// Runs a bounded async state-machine scenario selected by the input header.
pub fn run(data: &[u8]) {
    let stats = run_script(data);
    assert!(stats.polls <= POLL_LIMIT, "poll budget exhausted; input retained");
}

fn run_script(data: &[u8]) -> Stats {
    let (header, operations, events, payload) = parse(data);
    let text = String::from_utf8_lossy(&payload).into_owned();
    let event_count = events.len();
    let mut write_events = Vec::new();
    let mut flush_events = Vec::new();
    for (index, event) in events.iter().copied().enumerate() {
        if index % 2 == 0 {
            write_events.push(event);
        } else {
            flush_events.push(event);
        }
    }
    let mut stats = match header.scene {
        0 => run_writer(
            &text,
            header.capacity,
            &operations,
            write_events,
            flush_events,
            header.cancel,
        ),
        1 => run_reader(
            &text,
            header.capacity,
            &operations,
            events,
            false,
            header.endings,
            header.cancel,
        ),
        2 => run_reader(
            &text,
            header.capacity,
            &operations,
            events,
            true,
            header.endings,
            header.cancel,
        ),
        _ => run_reject_reader(&text, header.capacity, events, header.endings, header.cancel),
    };
    stats.events = header.event_count.min(event_count);
    stats
}

fn run_writer(
    text: &str,
    capacity: usize,
    ops: &[u8],
    write_events: Vec<Event>,
    flush_events: Vec<Event>,
    cancel: u8,
) -> Stats {
    let mut writer = AsyncCharsetTextWriter::new_with_buffer_capacity(
        ScriptOutput::new(write_events, flush_events),
        Utf8Codec,
        CharsetEncodePolicy::report(),
        capacity,
    );
    let mut stats = Stats::default();
    let mut offset = 0;
    let mut accepted = String::new();
    let mut polls = 0;
    let mut step = 0;
    while offset < text.len() {
        if stats.operations >= OP_LIMIT {
            break;
        }
        let operation = ops.get(step).copied().unwrap_or(0);
        stats.operations += 1;
        if operation % 4 == 3 {
            stats.finish_entries += 1;
            break;
        }
        let result = if operation % 4 == 2 {
            stats.write_char_ops += 1;
            let chars: Vec<char> = text[offset..].chars().collect();
            drive(writer.write_chars_async(&chars), &mut polls, cancel != 0)
                .map(|result| result.map(|count| chars.iter().take(count).map(|ch| ch.len_utf8()).sum()))
        } else {
            stats.write_str_ops += 1;
            drive(writer.write_str_async(&text[offset..]), &mut polls, cancel != 0)
        };
        let result = match result {
            Some(result) => result,
            None => {
                stats.cancelled += 1;
                continue;
            }
        };
        match result {
            Ok(count) => {
                assert!(count > 0 && text.is_char_boundary(offset + count));
                offset += count;
                accepted.push_str(&text[offset - count..offset]);
            }
            Err(error) if is_fault(&error, FaultKind::Write) => {
                stats.write_errors += 1;
                continue;
            }
            Err(error) => panic!("unexpected writer error: {error}"),
        }
        if operation & 1 != 0 {
            stats.flush_ops += 1;
            loop {
                match drive(writer.flush_async(), &mut polls, cancel != 0) {
                    None => stats.cancelled += 1,
                    Some(Ok(())) => break,
                    Some(Err(error)) if is_fault(&error, FaultKind::Write) => stats.write_errors += 1,
                    Some(Err(error)) if is_fault(&error, FaultKind::Flush) => stats.flush_errors += 1,
                    Some(Err(error)) => panic!("unexpected flush error: {error}"),
                }
            }
        }
        step += 1;
        let delivered = &writer.output().bytes;
        assert!(
            accepted.as_bytes().starts_with(delivered),
            "delivered output is not an accepted source prefix"
        );
        if polls >= POLL_LIMIT {
            panic!("poll budget exhausted in writer scenario");
        }
    }
    loop {
        match drive(writer.finish_async(), &mut polls, cancel != 0) {
            None => {
                stats.cancelled += 1;
                stats.finish_retries += 1;
            }
            Some(Ok(())) => break,
            Some(Err(error)) if is_fault(&error, FaultKind::Write) => {
                stats.write_errors += 1;
                stats.finish_retries += 1;
            }
            Some(Err(error)) if is_fault(&error, FaultKind::Flush) => {
                stats.flush_errors += 1;
                stats.finish_retries += 1;
            }
            Some(Err(error)) => panic!("unexpected finish error: {error}"),
        }
    }
    let io_stats = writer.output().stats;
    assert_eq!(
        block_on(writer.write_str_async(""), &mut polls).unwrap_err().kind(),
        io::ErrorKind::InvalidInput
    );
    assert_eq!(
        block_on(writer.write_str_async("x"), &mut polls).unwrap_err().kind(),
        io::ErrorKind::InvalidInput
    );
    let (output, pending) = writer.into_parts();
    assert_eq!(output.bytes, accepted.as_bytes());
    assert!(pending.is_empty());
    stats.pending = io_stats.pending;
    stats.write_errors = stats.write_errors.max(io_stats.write_errors);
    stats.flush_errors = stats.flush_errors.max(io_stats.flush_errors);
    stats.polls = polls;
    assert!(polls <= POLL_LIMIT, "poll budget exhausted in writer scenario");
    stats
}

fn run_reader(
    text: &str,
    capacity: usize,
    ops: &[u8],
    events: Vec<Event>,
    lines: bool,
    ending_index: usize,
    cancel: u8,
) -> Stats {
    let source = text.as_bytes().to_vec();
    let reader = AsyncCharsetTextReader::new_with_buffer_capacity(
        ScriptInput::new(source, events),
        Utf8Codec,
        qubit_codec_text::CharsetDecodePolicy::report(),
        capacity,
    );
    let mut stats = Stats::default();
    let mut polls = 0;
    if lines {
        let endings = line_ending_set(ending_index);
        let mut reader = reader.with_line_endings(endings);
        let expected = text_records(text, endings);
        let mut records = Vec::new();
        'reading: loop {
            if stats.line_ops >= OP_LIMIT {
                break;
            }
            let mut line = String::new();
            stats.line_ops += 1;
            stats.operations += 1;
            loop {
                match drive(reader.read_line_async(&mut line), &mut polls, cancel != 0) {
                    None => stats.cancelled += 1,
                    Some(Ok(true)) => {
                        records.push(line);
                        break;
                    }
                    Some(Ok(false)) => {
                        if !line.is_empty() {
                            records.push(line);
                            stats.eof_tail_records += 1;
                        }
                        stats.reader_eof = true;
                        break 'reading;
                    }
                    Some(Err(error)) if is_fault(&error, FaultKind::Read) => stats.read_errors += 1,
                    Some(Err(error)) => panic!("unexpected line error: {error}"),
                }
            }
            if polls > POLL_LIMIT {
                panic!("poll budget exhausted in line reader");
            }
        }
        assert_record_read_result(&expected, &records, stats.reader_eof);
        stats.pending = reader.input().stats.pending;
        stats.polls = polls;
    } else {
        let mut reader = reader;
        let mut result = String::new();
        let mut operation_index = 0;
        'reading: loop {
            if stats.operations >= OP_LIMIT {
                break;
            }
            let operation = ops.get(operation_index).copied().unwrap_or(0);
            operation_index += 1;
            stats.operations += 1;
            if operation & 1 == 0 {
                stats.read_char_ops += 1;
                loop {
                    match drive(reader.read_char_async(), &mut polls, cancel != 0) {
                        None => stats.cancelled += 1,
                        Some(Ok(Some(ch))) => {
                            result.push(ch);
                            break;
                        }
                        Some(Ok(None)) => {
                            stats.reader_eof = true;
                            break 'reading;
                        }
                        Some(Err(error)) if is_fault(&error, FaultKind::Read) => stats.read_errors += 1,
                        Some(Err(error)) => panic!("unexpected reader error: {error}"),
                    }
                }
            } else {
                stats.read_batch_ops += 1;
                let mut batch = Vec::new();
                loop {
                    match drive(reader.read_chars_async(&mut batch, 3), &mut polls, cancel != 0) {
                        None => stats.cancelled += 1,
                        Some(Ok(0)) => {
                            result.extend(batch);
                            stats.reader_eof = true;
                            break 'reading;
                        }
                        Some(Ok(_)) => {
                            result.extend(batch);
                            break;
                        }
                        Some(Err(error)) if is_fault(&error, FaultKind::Read) => {
                            stats.read_errors += 1;
                            result.extend(batch.drain(..));
                        }
                        Some(Err(error)) => panic!("unexpected reader error: {error}"),
                    }
                }
            }
            if polls > POLL_LIMIT {
                panic!("poll budget exhausted in char reader");
            }
        }
        assert_text_read_result(text, &result, stats.reader_eof);
        stats.pending = reader.input().stats.pending;
        stats.polls = polls;
    }
    assert!(polls <= POLL_LIMIT, "poll budget exhausted in reader scenario");
    stats
}

fn run_reject_reader(text: &str, capacity: usize, events: Vec<Event>, endings: usize, cancel: u8) -> Stats {
    let endings = line_ending_set(endings);
    let mut reader = AsyncCharsetTextReader::new_with_buffer_capacity(
        ScriptInput::new(text.as_bytes().to_vec(), events),
        Utf8Codec,
        qubit_codec_text::CharsetDecodePolicy::report(),
        capacity,
    )
    .with_line_endings(endings);
    let expected_records = text_records(text, endings);
    let mut seen = 0;
    let mut stats = Stats::default();
    let mut polls = 0;
    'reading: loop {
        if stats.line_ops >= OP_LIMIT {
            break;
        }
        stats.line_ops += 1;
        stats.operations += 1;
        let mut output = String::from("seed:中");
        loop {
            match drive(reader.read_line_limited_async(&mut output, 0), &mut polls, cancel != 0) {
                None => stats.cancelled += 1,
                Some(Ok(true)) => panic!("zero limit unexpectedly accepted record"),
                Some(Ok(false)) => {
                    stats.reader_eof = true;
                    break 'reading;
                }
                Some(Err(error)) if error.kind() == io::ErrorKind::InvalidData => {
                    assert_eq!(output, "seed:中");
                    seen += 1;
                    stats.rejected_records = seen;
                    break;
                }
                Some(Err(error)) if is_fault(&error, FaultKind::Read) => stats.read_errors += 1,
                Some(Err(error)) => panic!("unexpected rejected-line error: {error}"),
            }
        }
        if polls > POLL_LIMIT {
            panic!("poll budget exhausted in reject reader");
        }
    }
    assert_rejected_record_count(expected_records.len(), seen, stats.reader_eof);
    assert!(polls <= POLL_LIMIT, "poll budget exhausted in reject reader");
    stats.pending = reader.input().stats.pending;
    stats.polls = polls;
    stats
}

fn line_ending_set(index: usize) -> LineEndingSet {
    match index {
        0 => LineEndingSet::ALL,
        1 => LineEndingSet::LF,
        2 => LineEndingSet::CRLF,
        _ => LineEndingSet::CR,
    }
}

fn text_records(text: &str, set: LineEndingSet) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut result = Vec::new();
    let mut start = 0;
    let mut index = 0;
    while index < chars.len() {
        let ending_len = if chars[index] == '\r'
            && set.contains(qubit_io_text::LineEnding::CrLf)
            && chars.get(index + 1) == Some(&'\n')
        {
            2
        } else if chars[index] == '\n' && set.contains(qubit_io_text::LineEnding::Lf) {
            1
        } else if chars[index] == '\r' && set.contains(qubit_io_text::LineEnding::Cr) {
            1
        } else {
            0
        };
        if ending_len > 0 {
            result.push(chars[start..index + ending_len].iter().collect());
            index += ending_len;
            start = index;
        } else {
            index += 1;
        }
    }
    if start < chars.len() {
        result.push(chars[start..].iter().collect());
    }
    result
}

fn is_fault(error: &io::Error, kind: FaultKind) -> bool {
    error
        .get_ref()
        .and_then(|source| source.downcast_ref::<InjectedFault>())
        .is_some_and(|fault| fault.kind as u8 == kind as u8)
}

fn block_on<F: std::future::Future>(future: F, polls: &mut usize) -> F::Output {
    drive(future, polls, false).expect("non-cancelling driver always returns a result")
}

fn drive<F: std::future::Future>(future: F, polls: &mut usize, cancel_pending: bool) -> Option<F::Output> {
    let waker = std::task::Waker::noop();
    let mut context = Context::from_waker(waker);
    let mut future = std::pin::pin!(future);
    loop {
        *polls += 1;
        assert!(*polls <= POLL_LIMIT, "poll budget exhausted; async input retained");
        match future.as_mut().poll(&mut context) {
            Poll::Ready(result) => return Some(result),
            Poll::Pending if cancel_pending => return None,
            Poll::Pending => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::AtomicUsize;
    use std::sync::atomic::Ordering;

    use super::AsyncCharsetTextWriter;
    use super::AsyncInput;
    use super::AsyncOutput;
    use super::CharsetEncodePolicy;
    use super::Context;
    use super::EVENT_LIMIT;
    use super::Event;
    use super::FaultKind;
    use super::INPUT_LIMIT;
    use super::OP_LIMIT;
    use super::Pin;
    use super::Poll;
    use super::ScriptInput;
    use super::ScriptOutput;
    use super::Utf8Codec;
    use super::assert_record_read_result;
    use super::assert_rejected_record_count;
    use super::assert_text_read_result;
    use super::block_on;
    use super::is_fault;
    use super::parse;
    use super::run_script;

    #[derive(Default)]
    struct WakeCounter(AtomicUsize);

    #[test]
    #[should_panic(expected = "EOF text incomplete")]
    fn test_eof_oracle_rejects_missing_text() {
        assert_text_read_result("A", "", true);
    }

    #[test]
    #[should_panic(expected = "EOF records incomplete")]
    fn test_eof_oracle_rejects_missing_records() {
        assert_record_read_result(&[String::from("A\n")], &[], true);
    }

    #[test]
    #[should_panic(expected = "EOF rejected records incomplete")]
    fn test_eof_oracle_rejects_missing_rejections() {
        assert_rejected_record_count(1, 0, true);
    }

    #[test]
    fn test_eof_oracle_accepts_complete_results_and_budget_prefixes() {
        assert_text_read_result("A中", "A中", true);
        assert_text_read_result("A中", "A", false);
        let records = vec![String::from("A\r\n"), String::from("中")];
        assert_record_read_result(&records, &records, true);
        assert_record_read_result(&records, &records[..1], false);
        assert_rejected_record_count(2, 2, true);
        assert_rejected_record_count(2, 1, false);
    }

    #[test]
    #[should_panic(expected = "reader text is not a source prefix")]
    fn test_eof_oracle_rejects_non_prefix_text() {
        assert_text_read_result("A", "B", false);
    }

    #[test]
    #[should_panic(expected = "reader records are not a source prefix")]
    fn test_eof_oracle_rejects_non_prefix_records() {
        assert_record_read_result(&[String::from("A")], &[String::from("B")], false);
    }

    #[test]
    #[should_panic(expected = "too many rejected records")]
    fn test_eof_oracle_rejects_excess_rejection_count() {
        assert_rejected_record_count(1, 2, false);
    }

    impl std::task::Wake for WakeCounter {
        fn wake(self: Arc<Self>) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
        fn wake_by_ref(self: &Arc<Self>) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn operation_and_event_header_counts_wrap_at_sixty_five() {
        let mut data = vec![0; 6 + OP_LIMIT + EVENT_LIMIT];
        data[3] = 64;
        data[4] = 64;
        let (_, operations, events, _) = parse(&data);
        assert_eq!(operations.len(), 64);
        assert_eq!(events.len(), 64);

        data[3] = 65;
        data[4] = 65;
        let (_, operations, events, payload) = parse(&data);
        assert!(operations.is_empty());
        assert!(events.is_empty());
        assert_eq!(payload.len(), data.len() - 6);
    }

    #[test]
    fn output_events_are_independent_and_bounded_at_indexed_offsets() {
        let mut output = ScriptOutput::new(
            vec![Event::Pending, Event::Error, Event::Ready(1)],
            vec![Event::Error, Event::Ready(8)],
        );
        let wake_counter = Arc::new(WakeCounter::default());
        let waker = std::task::Waker::from(wake_counter.clone());
        let mut context = Context::from_waker(&waker);
        let source = *b"abcd";
        // SAFETY: source range 2..4 and the model's indexed output are valid.
        assert!(matches!(
            unsafe { Pin::new(&mut output).poll_write_unchecked(&mut context, &source, 2, 2) },
            Poll::Pending
        ));
        assert!(output.bytes.is_empty());
        assert_eq!(output.stats.pending, 1);
        assert!(wake_counter.0.load(Ordering::SeqCst) > 0);
        // SAFETY: same valid source range.
        let write_error = unsafe { Pin::new(&mut output).poll_write_unchecked(&mut context, &source, 2, 2) };
        let write_error = match write_error {
            Poll::Ready(Err(error)) => error,
            other => panic!("expected write error, got {other:?}"),
        };
        assert!(is_fault(&write_error, FaultKind::Write));
        assert!(output.bytes.is_empty());
        // SAFETY: same valid source range.
        assert!(matches!(
            unsafe { Pin::new(&mut output).poll_write_unchecked(&mut context, &source, 2, 2) },
            Poll::Ready(Ok(1))
        ));
        assert_eq!(output.bytes, b"c");
        let flush_error = Pin::new(&mut output).poll_flush(&mut context);
        let flush_error = match flush_error {
            Poll::Ready(Err(error)) => error,
            other => panic!("expected flush error, got {other:?}"),
        };
        assert!(is_fault(&flush_error, FaultKind::Flush));
        assert_eq!(output.bytes, b"c");
        assert!(matches!(
            Pin::new(&mut output).poll_flush(&mut context),
            Poll::Ready(Ok(()))
        ));
        assert_eq!(output.stats.write_errors, 1);
        assert_eq!(output.stats.flush_errors, 1);
    }

    #[test]
    fn cancelled_write_and_finish_resume_the_same_writer() {
        let output = ScriptOutput::new(
            vec![Event::Ready(8)],
            vec![Event::Pending, Event::Pending, Event::Ready(8)],
        );
        let mut writer =
            AsyncCharsetTextWriter::new_with_buffer_capacity(output, Utf8Codec, CharsetEncodePolicy::report(), 1);
        let waker = std::task::Waker::noop();
        let mut context = Context::from_waker(&waker);
        let mut polls = 0;
        let text = "A中🙂";
        let mut offset = 0;
        while offset < text.len() {
            let count = block_on(writer.write_str_async(&text[offset..]), &mut polls).unwrap();
            assert!(count > 0 && text.is_char_boundary(offset + count));
            offset += count;
        }
        {
            let future = writer.flush_async();
            let mut future = std::pin::pin!(future);
            assert!(matches!(future.as_mut().poll(&mut context), Poll::Pending));
        }
        {
            let future = writer.finish_async();
            let mut future = std::pin::pin!(future);
            assert!(matches!(future.as_mut().poll(&mut context), Poll::Pending));
        }
        block_on(writer.finish_async(), &mut polls).unwrap();
        assert_eq!(writer.output().bytes, text.as_bytes());
        assert!(writer.into_parts().1.is_empty());
    }

    #[test]
    fn buffered_parts_complete_the_accepted_prefix_at_a_step_boundary() {
        let output = ScriptOutput::new(vec![Event::Ready(8)], vec![Event::Ready(8)]);
        let mut writer =
            AsyncCharsetTextWriter::new_with_buffer_capacity(output, Utf8Codec, CharsetEncodePolicy::report(), 1);
        let mut polls = 0;
        let accepted = "A中";
        let written = block_on(writer.write_str_async(accepted), &mut polls).unwrap();
        assert!(written > 0 && accepted.is_char_boundary(written));
        let accepted_prefix = &accepted[..written];
        let (output, pending) = writer.into_buffered_parts();
        let mut complete = output.bytes;
        complete.extend_from_slice(pending.readable());
        assert_eq!(complete, accepted_prefix.as_bytes());
    }

    #[test]
    fn schedule_consumes_pending_error_and_ready_once_without_side_effects() {
        let mut input = ScriptInput::new(vec![9, 8], vec![Event::Pending, Event::Error, Event::Ready(1)]);
        let wake_counter = Arc::new(WakeCounter::default());
        let waker = std::task::Waker::from(wake_counter.clone());
        let mut context = Context::from_waker(&waker);
        let mut output = [0_u8; 4];
        // SAFETY: offset and count describe a valid nonzero destination range.
        assert!(matches!(
            unsafe { Pin::new(&mut input).poll_read_unchecked(&mut context, &mut output, 2, 2) },
            Poll::Pending
        ));
        assert_eq!(input.position, 0);
        assert_eq!(input.stats.pending, 1);
        assert!(wake_counter.0.load(Ordering::SeqCst) > 0, "Pending must arrange a wake");
        // SAFETY: same valid destination range.
        let fault = unsafe { Pin::new(&mut input).poll_read_unchecked(&mut context, &mut output, 2, 2) };
        assert!(matches!(fault, Poll::Ready(Err(_))));
        assert_eq!(input.position, 0);
        assert_eq!(input.stats.read_errors, 1);
        // SAFETY: same valid destination range.
        assert!(matches!(
            unsafe { Pin::new(&mut input).poll_read_unchecked(&mut context, &mut output, 2, 2) },
            Poll::Ready(Ok(1))
        ));
        assert_eq!(&output[2..3], &[9]);
        assert_eq!(input.position, 1);
        assert!(matches!(
            unsafe { Pin::new(&mut input).poll_read_unchecked(&mut context, &mut output, 4, 0) },
            Poll::Ready(Ok(0))
        ));
        assert!(matches!(
            unsafe { Pin::new(&mut input).poll_read_unchecked(&mut context, &mut output, 2, 2) },
            Poll::Ready(Ok(1))
        ));
        assert_eq!(input.position, 2);
        assert!(matches!(
            unsafe { Pin::new(&mut input).poll_read_unchecked(&mut context, &mut output, 1, 2) },
            Poll::Ready(Ok(0))
        ));
    }

    #[test]
    fn deterministic_scenarios_exercise_each_model_mode() {
        // Writer schedule: Pending on write, then a short write; the remaining
        // events force independent write/flush error handling.
        let writer = run_script(&[
            0, 0, 0, 0, 6, 1, 1, 2, 0, 1, 2, 3, b'A', 0xe4, 0xb8, 0xad, 0xf0, 0x9f, 0x99, 0x82,
        ]);
        assert_eq!(writer.write_errors, 1);
        assert_eq!(writer.flush_errors, 1);
        assert!(writer.finish_retries > 0);
        assert!(writer.cancelled > 0);
        let flush_operation = run_script(&[0, 0, 0, 1, 6, 1, 1, 1, 2, 2, 1, 3, 0, b'A']);
        assert!(flush_operation.flush_ops > 0);
        assert!(flush_operation.flush_errors > 0);
        let char_slice = run_script(&[0, 0, 0, 1, 6, 1, 2, 1, 2, 0, 1, 2, 3, b'A', 0xe4, 0xb8, 0xad]);
        assert!(char_slice.write_char_ops > 0);
        let finish_entry = run_script(&[0, 0, 0, 1, 2, 0, 3, 0, 0, b'A']);
        assert_eq!(finish_entry.finish_entries, 1);
        assert_eq!(finish_entry.write_str_ops, 0);
        assert_eq!(finish_entry.operations, 1);
        assert_eq!(finish_entry.events, 2);
        // Reader batches and lines receive pending/error events before drain.
        let chars = run_script(&[1, 0, 0, 1, 4, 1, 1, 1, 2, 3, 0, b'A', b'\n']);
        assert!(chars.reader_eof);
        assert!(chars.cancelled > 0);
        assert!(chars.read_errors > 0);
        assert!(chars.read_batch_ops > 0);
        let single_char = run_script(&[1, 0, 0, 1, 4, 1, 0, 1, 2, 3, 0, b'A']);
        assert!(single_char.reader_eof);
        assert!(single_char.cancelled > 0);
        assert!(single_char.read_errors > 0);
        assert!(single_char.read_char_ops > 0);
        let lines = run_script(&[2, 0, 0, 0, 4, 1, 1, 2, 0, 0, b'A', b'\r', b'\n', b'B', b'\r']);
        assert!(lines.reader_eof);
        assert!(lines.cancelled > 0);
        assert!(lines.read_errors > 0);
        assert!(lines.line_ops > 0);
        let unterminated = run_script(&[2, 0, 0, 0, 0, 0, b't', b'a', b'i', b'l']);
        assert!(unterminated.reader_eof);
        assert!(unterminated.line_ops >= 2);
        // Zero-limit line recovery uses a fixed CRLF set and has a following record.
        let rejected = run_script(&[3, 0, 2, 0, 4, 1, 1, 2, 0, 0, b'A', b'\r', b'\n', b'B', b'\r', b'\n']);
        assert!(rejected.reader_eof);
        assert_eq!(rejected.rejected_records, 2);
        assert!(rejected.cancelled > 0);
        assert!(rejected.read_errors > 0);
    }

    #[test]
    fn test_reader_eof_is_observed_in_each_mode() {
        for scene in [1, 2, 3] {
            assert!(run_script(&[scene, 0, 0, 0, 0, 0]).reader_eof);
            assert!(run_script(&[scene, 0, 0, 0, 0, 0, b'A']).reader_eof);
        }
        let batch = run_script(&[1, 0, 0, 1, 0, 0, 1, b'A']);
        assert!(batch.reader_eof);
        assert!(batch.read_batch_ops > 0);
        let tail = run_script(&[2, 0, 0, 0, 0, 0, b'A']);
        assert!(tail.reader_eof);
        assert_eq!(tail.line_ops, 2);
    }

    #[test]
    fn test_reader_eof_on_last_budgeted_operation_is_still_eof() {
        let mut input = vec![1, 0, 0, 0, 0, 0];
        input.extend(std::iter::repeat_n(b'A', OP_LIMIT - 1));
        let stats = run_script(&input);
        assert_eq!(stats.operations, OP_LIMIT);
        assert!(stats.reader_eof);
    }

    #[test]
    fn test_reader_budget_does_not_imply_eof() {
        for scene in [1, 2, 3] {
            let mut input = vec![scene, 0, 0, 0, 0, 0];
            input.extend(std::iter::repeat_n([b'A', b'\n'], OP_LIMIT + 1).flatten());
            let stats = run_script(&input);
            assert_eq!(stats.operations, OP_LIMIT);
            assert!(!stats.reader_eof);
        }
    }

    #[test]
    fn scenarios_stop_after_sixty_four_logical_operations() {
        let mut writer_input = vec![0, 0, 0, 65, 0, 0];
        writer_input.extend(std::iter::repeat_n(b'A', INPUT_LIMIT - 6));
        let writer = run_script(&writer_input);
        assert_eq!(OP_LIMIT, writer.operations);

        let mut reader_input = vec![1, 0, 0, 65, 0, 0];
        reader_input.extend(std::iter::repeat_n(b'A', OP_LIMIT + 1));
        let reader = run_script(&reader_input);
        assert_eq!(OP_LIMIT, reader.operations);

        let mut line_input = vec![2, 0, 0, 65, 0, 0];
        line_input.extend(std::iter::repeat_n(b'A', OP_LIMIT + 1).flat_map(|byte| [byte, b'\n']));
        let lines = run_script(&line_input);
        assert_eq!(OP_LIMIT, lines.operations);
        assert_eq!(OP_LIMIT, lines.line_ops);
    }

    #[test]
    fn minimized_nine_byte_line_error_seed_records_target_at_eof() {
        let stats = run_script(&[10, 255, 10, 0, 132, 249, 255, 10, 252]);
        assert!(stats.read_errors > 0);
        assert_eq!(stats.eof_tail_records, 1);
    }

    #[test]
    fn cancelled_line_resume_records_retained_target_at_eof() {
        // The Pending event cancels with a nonempty target; an injected read
        // error and then EOF exercise the per-call false-return edge.
        let stats = run_script(&[10, 255, 10, 0, 3, 249, 0, 1, 2, b'\n']);
        assert!(stats.cancelled > 0);
        assert!(stats.read_errors > 0);
        assert_eq!(stats.eof_tail_records, 1);
    }
}
