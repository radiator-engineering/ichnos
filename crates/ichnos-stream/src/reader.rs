//! Lazy CSV and XES iterators corresponding to pm4py streaming importers.

use crate::{Error, Result, StreamSink};
use ichnos_core::{Event, Trace};
use quick_xml::{Reader, Writer, events::Event as XmlEvent};
use std::{
    fmt,
    fs::File,
    io::{BufRead, BufReader, Read},
    path::{Path, PathBuf},
};

/// Deliver an iterator of fallible items to a consumer, stopping on error.
/// Rust iterator adapters supply transformations and acceptance conditions.
pub fn feed<T>(
    items: impl IntoIterator<Item = Result<T>>,
    sink: &mut impl StreamSink<T>,
) -> Result<usize> {
    let mut count = 0;
    for item in items {
        sink.push(&item?)?;
        count += 1;
    }
    Ok(count)
}

/// CSV parsing settings. Fields remain strings, including empty fields.
#[derive(Debug, Clone, Copy)]
pub struct CsvStreamOptions {
    /// Field delimiter; defaults to a comma.
    pub delimiter: u8,
}

impl Default for CsvStreamOptions {
    fn default() -> Self {
        Self { delimiter: b',' }
    }
}

/// Lazy header-named CSV events, following CSVEventStreamReader.
/// Unequal record widths and malformed CSV return errors rather than Python
/// DictReader's missing/extra-field placeholders. Readers fuse after an error.
pub struct CsvEventReader {
    reader: csv::Reader<Box<dyn Read>>,
    path: Option<PathBuf>,
    options: CsvStreamOptions,
    finished: bool,
}

impl fmt::Debug for CsvEventReader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CsvEventReader")
            .field("path", &self.path)
            .field("options", &self.options)
            .field("finished", &self.finished)
            .finish()
    }
}

impl CsvEventReader {
    /// Open a file. Reset reopens this path and closes the previous handle.
    pub fn open(path: impl AsRef<Path>, options: CsvStreamOptions) -> Result<Self> {
        let path = path.as_ref().to_owned();
        let mut reader = Self::from_reader(File::open(&path)?, options);
        reader.path = Some(path);
        Ok(reader)
    }

    /// Parse an owned reader; reset is unavailable without a path.
    pub fn from_reader(reader: impl Read + 'static, options: CsvStreamOptions) -> Self {
        Self {
            reader: csv::ReaderBuilder::new()
                .delimiter(options.delimiter)
                .from_reader(Box::new(reader)),
            path: None,
            options,
            finished: false,
        }
    }

    /// Reopen the original path at its header row.
    pub fn reset(&mut self) -> Result<()> {
        let fresh = Self::open(
            self.path.as_ref().ok_or(Error::NotRewindable)?,
            self.options,
        )?;
        *self = fresh;
        Ok(())
    }

    /// Read the next CSV event, preserving file order.
    pub fn read_event(&mut self) -> Result<Option<Event>> {
        self.next().transpose()
    }

    /// Send remaining events to a consumer.
    pub fn to_event_stream(&mut self, sink: &mut impl StreamSink) -> Result<usize> {
        feed(self, sink)
    }
}

impl Iterator for CsvEventReader {
    type Item = Result<Event>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }
        let read = (|| {
            let headers = self.reader.headers()?.clone();
            let mut record = csv::StringRecord::new();
            if !self.reader.read_record(&mut record)? {
                return Ok(None);
            }
            let mut event = Event::new();
            for (key, value) in headers.iter().zip(record.iter()) {
                event.insert(key, value);
            }
            Ok(Some(event))
        })();
        match read {
            Ok(Some(event)) => Some(Ok(event)),
            Ok(None) => {
                self.finished = true;
                None
            }
            Err(error) => {
                self.finished = true;
                Some(Err(error))
            }
        }
    }
}

impl std::iter::FusedIterator for CsvEventReader {}

/// Settings for incremental XES parsing, with file order retained.
#[derive(Debug, Clone)]
pub struct XesStreamOptions {
    /// Maximum XML nesting, including log/trace/event; defaults to 128.
    pub max_depth: usize,
    /// Prefix for trace attributes inherited by event streams; defaults to case:.
    pub case_prefix: String,
}

impl Default for XesStreamOptions {
    fn default() -> Self {
        Self {
            max_depth: 128,
            case_prefix: "case:".into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Event,
    Trace,
}

#[derive(Debug, Clone, Copy)]
enum CaptureKind {
    Metadata,
    Item,
}

struct Capture {
    depth: usize,
    kind: CaptureKind,
    bytes: Vec<u8>,
}

struct XesFrames {
    reader: Reader<Box<dyn BufRead>>,
    buffer: Vec<u8>,
    path: Option<PathBuf>,
    options: XesStreamOptions,
    mode: Mode,
    depth: usize,
    trace_depth: Option<usize>,
    prefix: Vec<u8>,
    capture: Option<Capture>,
    root_seen: bool,
    root_closed: bool,
    finished: bool,
}

impl fmt::Debug for XesFrames {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("XesFrames")
            .field("path", &self.path)
            .field("mode", &self.mode)
            .field("options", &self.options)
            .field("finished", &self.finished)
            .finish()
    }
}

impl XesFrames {
    fn from_reader(reader: impl BufRead + 'static, options: XesStreamOptions, mode: Mode) -> Self {
        let mut reader = Reader::from_reader(Box::new(reader) as Box<dyn BufRead>);
        reader.config_mut().expand_empty_elements = true;
        Self {
            reader,
            buffer: Vec::new(),
            path: None,
            options,
            mode,
            depth: 0,
            trace_depth: None,
            prefix: Vec::new(),
            capture: None,
            root_seen: false,
            root_closed: false,
            finished: false,
        }
    }

    fn open(path: impl AsRef<Path>, options: XesStreamOptions, mode: Mode) -> Result<Self> {
        let path = path.as_ref().to_owned();
        let input: Box<dyn BufRead> = if path.extension().is_some_and(|ext| ext == "gz") {
            Box::new(BufReader::new(flate2::read::MultiGzDecoder::new(
                File::open(&path)?,
            )))
        } else {
            Box::new(BufReader::new(File::open(&path)?))
        };
        let mut frames = Self::from_reader(input, options, mode);
        frames.path = Some(path);
        Ok(frames)
    }

    fn reset(&mut self) -> Result<()> {
        *self = Self::open(
            self.path.as_ref().ok_or(Error::NotRewindable)?,
            self.options.clone(),
            self.mode,
        )?;
        Ok(())
    }

    fn write(bytes: &mut Vec<u8>, token: &XmlEvent<'_>) -> Result<()> {
        Writer::new(bytes).write_event(token.clone())?;
        Ok(())
    }

    fn decode(&self, bytes: Vec<u8>) -> Result<Trace> {
        let mut document = b"<log>".to_vec();
        if self.mode == Mode::Event {
            document.extend_from_slice(b"<trace>");
            document.extend_from_slice(&self.prefix);
        }
        document.extend_from_slice(&bytes);
        if self.mode == Mode::Event {
            document.extend_from_slice(b"</trace>");
        }
        document.extend_from_slice(b"</log>");
        let options = ichnos_io::XesReadOptions {
            max_depth: self.options.max_depth,
            ..Default::default()
        };
        let mut log = ichnos_io::read_xes_from_reader(std::io::Cursor::new(document), &options)?;
        Ok(log
            .traces
            .pop()
            .expect("framed item always contains one trace"))
    }

    fn next_inner(&mut self) -> Result<Option<Trace>> {
        loop {
            self.buffer.clear();
            let token = self.reader.read_event_into(&mut self.buffer)?.into_owned();
            match &token {
                XmlEvent::Start(element) => {
                    self.depth += 1;
                    if self.depth > self.options.max_depth {
                        return Err(Error::Xes("nesting limit exceeded"));
                    }
                    let name = element.local_name();
                    let name = name.as_ref();
                    if self.depth == 1 {
                        if name != "log" || self.root_seen {
                            return Err(Error::Xes("expected a single log root"));
                        }
                        self.root_seen = true;
                    }
                    if let Some(capture) = &mut self.capture {
                        Self::write(&mut capture.bytes, &token)?;
                    } else if name == "trace" && self.depth == 2 {
                        self.trace_depth = Some(self.depth);
                        self.prefix.clear();
                        if self.mode == Mode::Trace {
                            let mut bytes = Vec::new();
                            Self::write(&mut bytes, &token)?;
                            self.capture = Some(Capture {
                                depth: self.depth,
                                kind: CaptureKind::Item,
                                bytes,
                            });
                        }
                    } else if self.mode == Mode::Event && self.trace_depth == Some(self.depth - 1) {
                        let mut bytes = Vec::new();
                        Self::write(&mut bytes, &token)?;
                        self.capture = Some(Capture {
                            depth: self.depth,
                            kind: if name == "event" {
                                CaptureKind::Item
                            } else {
                                CaptureKind::Metadata
                            },
                            bytes,
                        });
                    } else if name == "event" && self.trace_depth.is_none() {
                        return Err(Error::Xes("event outside a trace"));
                    }
                }
                XmlEvent::End(element) => {
                    let mut completed = None;
                    if let Some(capture) = &mut self.capture {
                        Self::write(&mut capture.bytes, &token)?;
                        if capture.depth == self.depth {
                            completed = self.capture.take();
                        }
                    }
                    let output = if let Some(capture) = completed {
                        match capture.kind {
                            CaptureKind::Metadata => {
                                self.prefix.extend_from_slice(&capture.bytes);
                                None
                            }
                            CaptureKind::Item => Some(self.decode(capture.bytes)?),
                        }
                    } else {
                        None
                    };
                    if element.local_name().as_ref() == "trace"
                        && self.trace_depth == Some(self.depth)
                    {
                        self.trace_depth = None;
                        self.prefix.clear();
                    }
                    if self.depth == 1 {
                        self.root_closed = true;
                    }
                    self.depth = self
                        .depth
                        .checked_sub(1)
                        .ok_or(Error::Xes("unexpected closing element"))?;
                    if output.is_some() {
                        return Ok(output);
                    }
                }
                XmlEvent::Text(text) if self.depth == 0 && !text.as_ref().trim().is_empty() => {
                    return Err(Error::Xes("text outside log root"));
                }
                XmlEvent::GeneralRef(_) if self.depth == 0 => {
                    return Err(Error::Xes("reference outside log root"));
                }
                XmlEvent::Eof => {
                    self.finished = true;
                    if !self.root_seen || !self.root_closed || self.depth != 0 {
                        return Err(Error::Xes("missing or truncated log root"));
                    }
                    return Ok(None);
                }
                _ => {
                    if let Some(capture) = &mut self.capture {
                        Self::write(&mut capture.bytes, &token)?;
                    }
                }
            }
        }
    }

    fn next_item(&mut self) -> Result<Option<Trace>> {
        if self.finished {
            return Ok(None);
        }
        let result = self.next_inner();
        if result.is_err() {
            self.finished = true;
        }
        result
    }
}

/// An event-at-a-time XES iterator, including inherited trace attributes.
/// Uses the merged XES attribute parser and retains at most one event plus
/// preceding trace metadata. XML/value errors are returned and fuse iteration.
#[derive(Debug)]
pub struct XesEventReader {
    frames: XesFrames,
}

impl XesEventReader {
    /// Open plain or gzip XES, preserving file order.
    pub fn open(path: impl AsRef<Path>, options: XesStreamOptions) -> Result<Self> {
        Ok(Self {
            frames: XesFrames::open(path, options, Mode::Event)?,
        })
    }
    /// Parse an owned buffered input without a resettable path.
    pub fn from_reader(input: impl BufRead + 'static, options: XesStreamOptions) -> Self {
        Self {
            frames: XesFrames::from_reader(input, options, Mode::Event),
        }
    }
    /// Reopen the original path and clear parsing state.
    pub fn reset(&mut self) -> Result<()> {
        self.frames.reset()
    }
    /// Read one event and overwrite case-prefixed fields with trace metadata.
    pub fn read_event(&mut self) -> Result<Option<Event>> {
        let Some(mut trace) = self.frames.next_item()? else {
            return Ok(None);
        };
        let mut event = trace.events.pop().expect("event frame contains one event");
        for (key, value) in trace.attributes.iter() {
            event.insert(
                format!("{}{key}", self.frames.options.case_prefix),
                value.clone(),
            );
        }
        Ok(Some(event))
    }
    /// Send remaining events to a consumer.
    pub fn to_event_stream(&mut self, sink: &mut impl StreamSink) -> Result<usize> {
        feed(self, sink)
    }
}

impl Iterator for XesEventReader {
    type Item = Result<Event>;
    fn next(&mut self) -> Option<Self::Item> {
        self.read_event().transpose()
    }
}
impl std::iter::FusedIterator for XesEventReader {}

/// A trace-at-a-time XES iterator retaining at most one complete trace.
#[derive(Debug)]
pub struct XesTraceReader {
    frames: XesFrames,
}

impl XesTraceReader {
    /// Open plain or gzip XES, retaining empty traces and file order.
    pub fn open(path: impl AsRef<Path>, options: XesStreamOptions) -> Result<Self> {
        Ok(Self {
            frames: XesFrames::open(path, options, Mode::Trace)?,
        })
    }
    /// Parse an owned buffered input without a resettable path.
    pub fn from_reader(input: impl BufRead + 'static, options: XesStreamOptions) -> Self {
        Self {
            frames: XesFrames::from_reader(input, options, Mode::Trace),
        }
    }
    /// Reopen the original path and clear parsing state.
    pub fn reset(&mut self) -> Result<()> {
        self.frames.reset()
    }
    /// Read the next complete trace.
    pub fn read_trace(&mut self) -> Result<Option<Trace>> {
        self.frames.next_item()
    }
    /// Send remaining traces to a consumer.
    pub fn to_trace_stream(&mut self, sink: &mut impl StreamSink<Trace>) -> Result<usize> {
        feed(self, sink)
    }
}

impl Iterator for XesTraceReader {
    type Item = Result<Trace>;
    fn next(&mut self) -> Option<Self::Item> {
        self.read_trace().transpose()
    }
}
impl std::iter::FusedIterator for XesTraceReader {}
