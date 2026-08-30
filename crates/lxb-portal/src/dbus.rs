use std::collections::VecDeque;
use std::io::{ErrorKind, Read, Write};
use std::os::linux::net::SocketAddrExt;
use std::os::unix::fs::MetadataExt;
use std::os::unix::net::{SocketAddr, UnixStream};
use std::time::{Duration, Instant};

pub const METHOD_CALL: u8 = 1;
pub const METHOD_RETURN: u8 = 2;
pub const ERROR: u8 = 3;
pub const SIGNAL: u8 = 4;

const MOST_HELD: usize = 256;
const MOST_IN_ONE_MESSAGE: usize = 128 * 1024 * 1024;
const WAKE: Duration = Duration::from_millis(100);

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Byte(u8),
    Bool(bool),
    I16(i16),
    U16(u16),
    I32(i32),
    U32(u32),
    I64(i64),
    U64(u64),
    F64(f64),
    Str(String),
    Path(String),
    Sig(String),
    Array(String, Vec<Value>),
    Struct(Vec<Value>),
    Pair(Box<Value>, Box<Value>),
    Variant(Box<Value>),
}

impl Value {
    pub fn text(text: impl Into<String>) -> Self {
        Value::Str(text.into())
    }

    pub fn options(entries: Vec<(&str, Value)>) -> Self {
        Value::Array(
            "{sv}".to_string(),
            entries
                .into_iter()
                .map(|(key, value)| {
                    Value::Pair(
                        Box::new(Value::Str(key.to_string())),
                        Box::new(Value::Variant(Box::new(value))),
                    )
                })
                .collect(),
        )
    }

    pub fn bytes(raw: &[u8]) -> Self {
        Value::Array(
            "y".to_string(),
            raw.iter().copied().map(Value::Byte).collect(),
        )
    }

    pub fn signature(&self) -> String {
        match self {
            Value::Byte(_) => "y".to_string(),
            Value::Bool(_) => "b".to_string(),
            Value::I16(_) => "n".to_string(),
            Value::U16(_) => "q".to_string(),
            Value::I32(_) => "i".to_string(),
            Value::U32(_) => "u".to_string(),
            Value::I64(_) => "x".to_string(),
            Value::U64(_) => "t".to_string(),
            Value::F64(_) => "d".to_string(),
            Value::Str(_) => "s".to_string(),
            Value::Path(_) => "o".to_string(),
            Value::Sig(_) => "g".to_string(),
            Value::Array(element, _) => format!("a{element}"),
            Value::Struct(fields) => {
                let mut out = String::from("(");
                for field in fields {
                    out.push_str(&field.signature());
                }
                out.push(')');
                out
            }
            Value::Pair(key, value) => {
                format!("{{{}{}}}", key.signature(), value.signature())
            }
            Value::Variant(_) => "v".to_string(),
        }
    }

    pub fn peeled(&self) -> &Value {
        match self {
            Value::Variant(inner) => inner.peeled(),
            other => other,
        }
    }

    pub fn as_u32(&self) -> Option<u32> {
        match self.peeled() {
            Value::U32(number) => Some(*number),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self.peeled() {
            Value::Str(text) | Value::Path(text) | Value::Sig(text) => Some(text),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<&[Value]> {
        match self.peeled() {
            Value::Array(_, items) => Some(items),
            _ => None,
        }
    }

    pub fn as_strings(&self) -> Option<Vec<String>> {
        self.as_list().map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_string))
                .collect()
        })
    }

    pub fn at(&self, key: &str) -> Option<&Value> {
        let items = self.as_list()?;
        items.iter().find_map(|item| match item {
            Value::Pair(name, value) if name.as_str() == Some(key) => Some(value.as_ref()),
            _ => None,
        })
    }
}

fn alignment(signature: &str) -> usize {
    match signature.as_bytes().first() {
        Some(b'y' | b'g' | b'v') => 1,
        Some(b'n' | b'q') => 2,
        Some(b'x' | b't' | b'd' | b'(' | b'{') => 8,
        _ => 4,
    }
}

fn split_type(signature: &str) -> Option<(&str, &str)> {
    let bytes = signature.as_bytes();
    let mut at = 0;
    while bytes.get(at) == Some(&b'a') {
        at += 1;
    }
    match *bytes.get(at)? {
        opening @ (b'(' | b'{') => {
            let closing = if opening == b'(' { b')' } else { b'}' };
            let mut depth = 0usize;
            loop {
                let byte = *bytes.get(at)?;
                if byte == opening {
                    depth += 1;
                } else if byte == closing {
                    depth -= 1;
                }
                at += 1;
                if depth == 0 {
                    break;
                }
            }
        }
        _ => at += 1,
    }
    Some(signature.split_at(at))
}

fn every_type(signature: &str) -> Result<Vec<&str>, String> {
    let mut rest = signature;
    let mut types = Vec::new();
    while !rest.is_empty() {
        let (one, left) = split_type(rest)
            .ok_or_else(|| format!("a message named a type this program cannot read: {rest}"))?;
        types.push(one);
        rest = left;
    }
    Ok(types)
}

#[derive(Default)]
struct Writer {
    out: Vec<u8>,
}

impl Writer {
    fn pad(&mut self, to: usize) {
        while !self.out.len().is_multiple_of(to) {
            self.out.push(0);
        }
    }

    fn raw(&mut self, bytes: &[u8]) {
        self.out.extend_from_slice(bytes);
    }

    fn text(&mut self, text: &str) {
        self.pad(4);
        self.raw(&(text.len() as u32).to_le_bytes());
        self.raw(text.as_bytes());
        self.out.push(0);
    }

    fn signature(&mut self, signature: &str) {
        self.out.push(signature.len() as u8);
        self.raw(signature.as_bytes());
        self.out.push(0);
    }

    fn value(&mut self, value: &Value) {
        match value {
            Value::Byte(number) => self.out.push(*number),
            Value::Bool(yes) => {
                self.pad(4);
                self.raw(&u32::from(*yes).to_le_bytes());
            }
            Value::I16(number) => {
                self.pad(2);
                self.raw(&number.to_le_bytes());
            }
            Value::U16(number) => {
                self.pad(2);
                self.raw(&number.to_le_bytes());
            }
            Value::I32(number) => {
                self.pad(4);
                self.raw(&number.to_le_bytes());
            }
            Value::U32(number) => {
                self.pad(4);
                self.raw(&number.to_le_bytes());
            }
            Value::I64(number) => {
                self.pad(8);
                self.raw(&number.to_le_bytes());
            }
            Value::U64(number) => {
                self.pad(8);
                self.raw(&number.to_le_bytes());
            }
            Value::F64(number) => {
                self.pad(8);
                self.raw(&number.to_le_bytes());
            }
            Value::Str(text) | Value::Path(text) => self.text(text),
            Value::Sig(signature) => self.signature(signature),
            Value::Array(element, items) => {
                self.pad(4);
                let counted_at = self.out.len();
                self.raw(&0u32.to_le_bytes());
                self.pad(alignment(element));
                let from = self.out.len();
                for item in items {
                    self.value(item);
                }
                let length = (self.out.len() - from) as u32;
                self.out[counted_at..counted_at + 4].copy_from_slice(&length.to_le_bytes());
            }
            Value::Struct(fields) => {
                self.pad(8);
                for field in fields {
                    self.value(field);
                }
            }
            Value::Pair(key, value) => {
                self.pad(8);
                self.value(key);
                self.value(value);
            }
            Value::Variant(inner) => {
                self.signature(&inner.signature());
                self.value(inner);
            }
        }
    }
}

struct Reader<'a> {
    data: &'a [u8],
    at: usize,
    little: bool,
}

impl<'a> Reader<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8], String> {
        let end = self
            .at
            .checked_add(count)
            .ok_or_else(|| "a message named a length no message could hold".to_string())?;
        let slice = self
            .data
            .get(self.at..end)
            .ok_or_else(|| "a message ended in the middle of a value".to_string())?;
        self.at = end;
        Ok(slice)
    }

    fn pad(&mut self, to: usize) -> Result<(), String> {
        while !self.at.is_multiple_of(to) {
            self.take(1)?;
        }
        Ok(())
    }

    fn byte(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, String> {
        self.pad(2)?;
        let raw: [u8; 2] = self.take(2)?.try_into().expect("two bytes");
        Ok(if self.little {
            u16::from_le_bytes(raw)
        } else {
            u16::from_be_bytes(raw)
        })
    }

    fn u32(&mut self) -> Result<u32, String> {
        self.pad(4)?;
        let raw: [u8; 4] = self.take(4)?.try_into().expect("four bytes");
        Ok(if self.little {
            u32::from_le_bytes(raw)
        } else {
            u32::from_be_bytes(raw)
        })
    }

    fn u64(&mut self) -> Result<u64, String> {
        self.pad(8)?;
        let raw: [u8; 8] = self.take(8)?.try_into().expect("eight bytes");
        Ok(if self.little {
            u64::from_le_bytes(raw)
        } else {
            u64::from_be_bytes(raw)
        })
    }

    fn text(&mut self) -> Result<String, String> {
        let length = self.u32()? as usize;
        let raw = self.take(length)?.to_vec();
        self.take(1)?;
        String::from_utf8(raw).map_err(|_| "a message carried a name that is not text".to_string())
    }

    fn signature(&mut self) -> Result<String, String> {
        let length = self.byte()? as usize;
        let raw = self.take(length)?.to_vec();
        self.take(1)?;
        String::from_utf8(raw).map_err(|_| "a message carried a type that is not text".to_string())
    }

    fn value(&mut self, signature: &str) -> Result<Value, String> {
        let first = *signature
            .as_bytes()
            .first()
            .ok_or_else(|| "a message named an empty type".to_string())?;
        match first {
            b'y' => Ok(Value::Byte(self.byte()?)),
            b'b' => Ok(Value::Bool(self.u32()? != 0)),
            b'n' => Ok(Value::I16(self.u16()? as i16)),
            b'q' => Ok(Value::U16(self.u16()?)),
            b'i' => Ok(Value::I32(self.u32()? as i32)),
            b'u' | b'h' => Ok(Value::U32(self.u32()?)),
            b'x' => Ok(Value::I64(self.u64()? as i64)),
            b't' => Ok(Value::U64(self.u64()?)),
            b'd' => Ok(Value::F64(f64::from_bits(self.u64()?))),
            b's' => Ok(Value::Str(self.text()?)),
            b'o' => Ok(Value::Path(self.text()?)),
            b'g' => Ok(Value::Sig(self.signature()?)),
            b'v' => {
                let inner = self.signature()?;
                Ok(Value::Variant(Box::new(self.value(&inner)?)))
            }
            b'a' => {
                let element = &signature[1..];
                let length = self.u32()? as usize;
                self.pad(alignment(element))?;
                let end = self
                    .at
                    .checked_add(length)
                    .ok_or_else(|| "a message named a list no message could hold".to_string())?;
                if end > self.data.len() {
                    return Err("a message ended in the middle of a list".to_string());
                }
                let mut items = Vec::new();
                while self.at < end {
                    items.push(self.value(element)?);
                }
                if self.at != end {
                    return Err("a list overran the length it declared".to_string());
                }
                Ok(Value::Array(element.to_string(), items))
            }
            b'(' => {
                self.pad(8)?;
                let inner = &signature[1..signature.len().saturating_sub(1)];
                let mut fields = Vec::new();
                for one in every_type(inner)? {
                    fields.push(self.value(one)?);
                }
                Ok(Value::Struct(fields))
            }
            b'{' => {
                self.pad(8)?;
                let inner = &signature[1..signature.len().saturating_sub(1)];
                let types = every_type(inner)?;
                let (key, value) = match types.as_slice() {
                    [key, value] => (*key, *value),
                    _ => return Err("a pair had something other than two halves".to_string()),
                };
                let key = self.value(key)?;
                let value = self.value(value)?;
                Ok(Value::Pair(Box::new(key), Box::new(value)))
            }
            other => Err(format!(
                "a message named a type this program cannot read: {}",
                other as char
            )),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Message {
    pub kind: u8,
    pub path: Option<String>,
    pub interface: Option<String>,
    pub member: Option<String>,
    pub error: Option<String>,
    pub reply_serial: Option<u32>,
    pub destination: Option<String>,
    pub sender: Option<String>,
    pub body: Vec<Value>,
}

impl Message {
    pub fn call(destination: &str, path: &str, interface: &str, member: &str) -> Self {
        Self {
            kind: METHOD_CALL,
            destination: Some(destination.to_string()),
            path: Some(path.to_string()),
            interface: Some(interface.to_string()),
            member: Some(member.to_string()),
            ..Self::default()
        }
    }

    pub fn with(mut self, body: Vec<Value>) -> Self {
        self.body = body;
        self
    }

    pub fn is_signal(&self, path: &str, interface: &str, member: &str) -> bool {
        self.kind == SIGNAL
            && self.path.as_deref() == Some(path)
            && self.interface.as_deref() == Some(interface)
            && self.member.as_deref() == Some(member)
    }

    fn complaint(&self) -> String {
        let name = self.error.clone().unwrap_or_else(|| "an error".to_string());
        match self.body.first().and_then(Value::as_str) {
            Some(said) => format!("{name}: {said}"),
            None => name,
        }
    }

    fn encode(&self, serial: u32) -> Vec<u8> {
        let mut body = Writer::default();
        for value in &self.body {
            body.value(value);
        }
        let signature: String = self.body.iter().map(Value::signature).collect();

        let mut fields = Vec::new();
        let mut field = |code: u8, value: Value| {
            fields.push(Value::Struct(vec![
                Value::Byte(code),
                Value::Variant(Box::new(value)),
            ]));
        };
        if let Some(path) = &self.path {
            field(1, Value::Path(path.clone()));
        }
        if let Some(interface) = &self.interface {
            field(2, Value::Str(interface.clone()));
        }
        if let Some(member) = &self.member {
            field(3, Value::Str(member.clone()));
        }
        if let Some(error) = &self.error {
            field(4, Value::Str(error.clone()));
        }
        if let Some(reply) = self.reply_serial {
            field(5, Value::U32(reply));
        }
        if let Some(destination) = &self.destination {
            field(6, Value::Str(destination.clone()));
        }
        if !signature.is_empty() {
            field(8, Value::Sig(signature));
        }

        let mut out = Writer::default();
        out.raw(&[b'l', self.kind, 0, 1]);
        out.raw(&(body.out.len() as u32).to_le_bytes());
        out.raw(&serial.to_le_bytes());
        out.value(&Value::Array("(yv)".to_string(), fields));
        out.pad(8);
        out.raw(&body.out);
        out.out
    }

    fn decode(raw: &[u8]) -> Result<Self, String> {
        let little = match raw.first() {
            Some(b'l') => true,
            Some(b'B') => false,
            _ => return Err("a message arrived in no byte order at all".to_string()),
        };
        let mut reader = Reader {
            data: raw,
            at: 0,
            little,
        };
        reader.take(1)?;
        let kind = reader.byte()?;
        reader.byte()?;
        let version = reader.byte()?;
        if version != 1 {
            return Err(format!("a message spoke version {version} of the protocol"));
        }
        let _body_length = reader.u32()?;
        let _serial = reader.u32()?;

        let mut message = Self {
            kind,
            ..Self::default()
        };
        let mut signature = String::new();
        let fields = reader.value("a(yv)")?;
        for field in fields.as_list().unwrap_or_default() {
            let Value::Struct(halves) = field else {
                continue;
            };
            let (Some(Value::Byte(code)), Some(value)) = (halves.first(), halves.get(1)) else {
                continue;
            };
            let said = value.as_str().map(str::to_string);
            match code {
                1 => message.path = said,
                2 => message.interface = said,
                3 => message.member = said,
                4 => message.error = said,
                5 => message.reply_serial = value.as_u32(),
                6 => message.destination = said,
                7 => message.sender = said,
                8 => signature = said.unwrap_or_default(),
                _ => {}
            }
        }
        reader.pad(8)?;
        for one in every_type(&signature)? {
            message.body.push(reader.value(one)?);
        }
        Ok(message)
    }
}

pub struct Bus {
    stream: UnixStream,
    serial: u32,
    held: VecDeque<Message>,
    unique: String,
}

impl Bus {
    pub fn session() -> Result<Self, String> {
        let stream = connect(&address()?)?;
        stream
            .set_read_timeout(Some(WAKE))
            .map_err(|err| format!("the bus socket would not be timed: {err}"))?;
        let mut bus = Self {
            stream,
            serial: 0,
            held: VecDeque::new(),
            unique: String::new(),
        };
        bus.authenticate()?;
        let hello = bus.call(
            Message::call(
                "org.freedesktop.DBus",
                "/org/freedesktop/DBus",
                "org.freedesktop.DBus",
                "Hello",
            ),
            Duration::from_secs(5),
        )?;
        bus.unique = hello
            .body
            .first()
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        if bus.unique.is_empty() {
            return Err("the bus gave this program no name of its own".to_string());
        }
        Ok(bus)
    }

    pub fn unique_name(&self) -> &str {
        &self.unique
    }

    pub fn call(&mut self, message: Message, patience: Duration) -> Result<Message, String> {
        let deadline = Instant::now() + patience;
        self.serial += 1;
        let serial = self.serial;
        self.stream
            .write_all(&message.encode(serial))
            .map_err(|err| format!("the bus could not be written to: {err}"))?;
        loop {
            let answer = self.read_message(deadline)?;
            if answer.reply_serial != Some(serial) {
                self.hold(answer);
                continue;
            }
            if answer.kind == ERROR {
                return Err(answer.complaint());
            }
            if answer.kind == METHOD_RETURN {
                return Ok(answer);
            }
            self.hold(answer);
        }
    }

    pub fn wait_for(
        &mut self,
        deadline: Instant,
        mut wanted: impl FnMut(&Message) -> bool,
    ) -> Result<Message, String> {
        for _ in 0..self.held.len() {
            let Some(held) = self.held.pop_front() else {
                break;
            };
            if wanted(&held) {
                return Ok(held);
            }
            self.held.push_back(held);
        }
        loop {
            let message = self.read_message(deadline)?;
            if wanted(&message) {
                return Ok(message);
            }
            self.hold(message);
        }
    }

    fn hold(&mut self, message: Message) {
        if self.held.len() >= MOST_HELD {
            self.held.pop_front();
        }
        self.held.push_back(message);
    }

    fn authenticate(&mut self) -> Result<(), String> {
        let deadline = Instant::now() + Duration::from_secs(5);
        self.stream
            .write_all(&[0])
            .map_err(|err| format!("the bus could not be greeted: {err}"))?;
        let named: String = us()
            .to_string()
            .bytes()
            .map(|by| format!("{by:02x}"))
            .collect();
        self.say(&format!("AUTH EXTERNAL {named}"))?;
        let mut answer = self.read_line(deadline)?;
        if answer.starts_with("REJECTED") {
            self.say("AUTH EXTERNAL")?;
            answer = self.read_line(deadline)?;
            if answer.starts_with("DATA") {
                self.say("DATA")?;
                answer = self.read_line(deadline)?;
            }
        }
        if !answer.starts_with("OK") {
            return Err(format!("the bus would not have this program: {answer}"));
        }
        self.say("BEGIN")
    }

    fn say(&mut self, line: &str) -> Result<(), String> {
        self.stream
            .write_all(format!("{line}\r\n").as_bytes())
            .map_err(|err| format!("the bus could not be written to: {err}"))
    }

    fn read_line(&mut self, deadline: Instant) -> Result<String, String> {
        let mut line = Vec::new();
        loop {
            let byte = self.read_exactly(1, deadline)?[0];
            if byte == b'\n' {
                while line.last() == Some(&b'\r') {
                    line.pop();
                }
                return Ok(String::from_utf8_lossy(&line).into_owned());
            }
            line.push(byte);
            if line.len() > 4096 {
                return Err("the bus said more than a greeting can hold".to_string());
            }
        }
    }

    fn read_message(&mut self, deadline: Instant) -> Result<Message, String> {
        let head = self.read_exactly(16, deadline)?;
        let little = match head.first() {
            Some(b'l') => true,
            Some(b'B') => false,
            _ => return Err("a message arrived in no byte order at all".to_string()),
        };
        let number = |raw: &[u8]| -> u32 {
            let raw: [u8; 4] = raw.try_into().expect("four bytes");
            if little {
                u32::from_le_bytes(raw)
            } else {
                u32::from_be_bytes(raw)
            }
        };
        let body_length = number(&head[4..8]) as usize;
        let fields_length = number(&head[12..16]) as usize;
        if body_length > MOST_IN_ONE_MESSAGE || fields_length > MOST_IN_ONE_MESSAGE {
            return Err("a message declared more than this program will read".to_string());
        }
        let padded = (16 + fields_length).div_ceil(8) * 8;
        let rest = self.read_exactly(padded - 16 + body_length, deadline)?;
        let mut whole = head;
        whole.extend_from_slice(&rest);
        Message::decode(&whole)
    }

    fn read_exactly(&mut self, count: usize, deadline: Instant) -> Result<Vec<u8>, String> {
        let mut out = vec![0u8; count];
        let mut filled = 0;
        while filled < count {
            match self.stream.read(&mut out[filled..]) {
                Ok(0) => return Err("the bus closed the connection".to_string()),
                Ok(read) => filled += read,
                Err(err)
                    if matches!(
                        err.kind(),
                        ErrorKind::WouldBlock | ErrorKind::TimedOut | ErrorKind::Interrupted
                    ) =>
                {
                    if Instant::now() >= deadline {
                        return Err("the bus did not answer in time".to_string());
                    }
                }
                Err(err) => return Err(format!("the bus could not be read: {err}")),
            }
        }
        Ok(out)
    }
}

fn us() -> u32 {
    std::fs::metadata("/proc/self")
        .map(|facts| facts.uid())
        .unwrap_or(0)
}

fn address() -> Result<String, String> {
    match std::env::var("DBUS_SESSION_BUS_ADDRESS") {
        Ok(address) if !address.trim().is_empty() => return Ok(address),
        _ => {}
    }
    let path = format!("/run/user/{}/bus", us());
    if std::path::Path::new(&path).exists() {
        return Ok(format!("unix:path={path}"));
    }
    Err("this session has no message bus".to_string())
}

fn unescaped(text: &str) -> String {
    let raw = text.as_bytes();
    let mut out = Vec::with_capacity(raw.len());
    let mut at = 0;
    while at < raw.len() {
        if raw[at] == b'%' && at + 2 < raw.len() {
            let pair = std::str::from_utf8(&raw[at + 1..at + 3]).unwrap_or("");
            if let Ok(byte) = u8::from_str_radix(pair, 16) {
                out.push(byte);
                at += 3;
                continue;
            }
        }
        out.push(raw[at]);
        at += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn connect(address: &str) -> Result<UnixStream, String> {
    let mut refusal = "this session names no socket to reach the bus on".to_string();
    for one in address.split(';') {
        let Some(rest) = one.trim().strip_prefix("unix:") else {
            continue;
        };
        for part in rest.split(',') {
            let opened = if let Some(path) = part.strip_prefix("path=") {
                UnixStream::connect(unescaped(path))
            } else if let Some(name) = part.strip_prefix("abstract=") {
                SocketAddr::from_abstract_name(unescaped(name).as_bytes())
                    .and_then(|address| UnixStream::connect_addr(&address))
            } else {
                continue;
            };
            match opened {
                Ok(stream) => return Ok(stream),
                Err(err) => refusal = format!("the bus socket would not open: {err}"),
            }
        }
    }
    Err(refusal)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_type_is_split_off_a_signature_whole() {
        assert_eq!(split_type("ssa{sv}"), Some(("s", "sa{sv}")));
        assert_eq!(split_type("a{sv}u"), Some(("a{sv}", "u")));
        assert_eq!(split_type("a(sa(us))"), Some(("a(sa(us))", "")));
        assert_eq!(split_type("(s(ii))b"), Some(("(s(ii))", "b")));
        assert_eq!(split_type("aay"), Some(("aay", "")));
        assert_eq!(split_type(""), None);
        assert_eq!(split_type("aa"), None);
    }

    #[test]
    fn every_type_of_a_body_is_named_in_order() {
        assert_eq!(every_type("ua{sv}").unwrap(), ["u", "a{sv}"]);
        assert_eq!(every_type("").unwrap(), Vec::<&str>::new());
        assert!(every_type("a").is_err());
    }

    fn round_trip(value: &Value) -> Value {
        let mut writer = Writer::default();
        writer.value(value);
        let mut reader = Reader {
            data: &writer.out,
            at: 0,
            little: true,
        };
        reader
            .value(&value.signature())
            .expect("what was written can be read")
    }

    #[test]
    fn every_kind_of_value_survives_the_wire() {
        for value in [
            Value::Byte(7),
            Value::Bool(true),
            Value::U32(4_000_000_000),
            Value::I32(-5),
            Value::U64(u64::MAX),
            Value::F64(0.5),
            Value::Str("a name".to_string()),
            Value::Path("/org/freedesktop/portal/desktop".to_string()),
            Value::Sig("a{sv}".to_string()),
            Value::Variant(Box::new(Value::U32(4))),
            Value::Struct(vec![Value::Str("Images".to_string()), Value::U32(0)]),
            Value::Array("s".to_string(), vec![Value::Str("one".to_string())]),
            Value::Array("s".to_string(), Vec::new()),
        ] {
            assert_eq!(round_trip(&value), value, "{}", value.signature());
        }
    }

    #[test]
    fn the_portals_own_shapes_survive_the_wire() {
        let filters = Value::Array(
            "(sa(us))".to_string(),
            vec![Value::Struct(vec![
                Value::Str("Images".to_string()),
                Value::Array(
                    "(us)".to_string(),
                    vec![
                        Value::Struct(vec![Value::U32(0), Value::Str("*.png".to_string())]),
                        Value::Struct(vec![Value::U32(1), Value::Str("image/jpeg".to_string())]),
                    ],
                ),
            ])],
        );
        assert_eq!(filters.signature(), "a(sa(us))");
        assert_eq!(round_trip(&filters), filters);

        let options = Value::options(vec![
            ("handle_token", Value::text("lxb1_0")),
            ("multiple", Value::Bool(true)),
            ("current_folder", Value::bytes(b"/home/me\0")),
            ("filters", filters),
        ]);
        assert_eq!(options.signature(), "a{sv}");
        let back = round_trip(&options);
        assert_eq!(back, options);
        assert_eq!(
            back.at("handle_token").and_then(Value::as_str),
            Some("lxb1_0")
        );
        assert_eq!(back.at("nothing"), None);
    }

    #[test]
    fn a_call_and_its_answer_survive_the_wire() {
        let call = Message::call(
            "org.freedesktop.portal.Desktop",
            "/org/freedesktop/portal/desktop",
            "org.freedesktop.portal.FileChooser",
            "OpenFile",
        )
        .with(vec![
            Value::text(""),
            Value::text("Choose a file"),
            Value::options(vec![("modal", Value::Bool(true))]),
        ]);
        let raw = call.encode(42);
        let body_length = u32::from_le_bytes(raw[4..8].try_into().unwrap()) as usize;
        assert!(
            (raw.len() - body_length).is_multiple_of(8),
            "a body starts on an eight-byte boundary"
        );
        assert_eq!(
            u32::from_le_bytes(raw[8..12].try_into().unwrap()),
            42,
            "the serial the call was sent under"
        );
        let back = Message::decode(&raw).expect("a call this program wrote");
        assert_eq!(back.kind, METHOD_CALL);
        assert_eq!(back.member.as_deref(), Some("OpenFile"));
        assert_eq!(back.body.len(), 3);
        assert_eq!(back.body[1].as_str(), Some("Choose a file"));

        let mut answer = Message {
            kind: SIGNAL,
            path: Some("/org/freedesktop/portal/desktop/request/1_42/lxb1_0".to_string()),
            interface: Some("org.freedesktop.portal.Request".to_string()),
            member: Some("Response".to_string()),
            ..Message::default()
        };
        answer.body = vec![
            Value::U32(0),
            Value::options(vec![(
                "uris",
                Value::Array(
                    "s".to_string(),
                    vec![Value::text("file:///home/me/a%20file.png")],
                ),
            )]),
        ];
        let back = Message::decode(&answer.encode(9)).expect("a signal this program wrote");
        assert!(back.is_signal(
            "/org/freedesktop/portal/desktop/request/1_42/lxb1_0",
            "org.freedesktop.portal.Request",
            "Response"
        ));
        assert_eq!(back.body[0].as_u32(), Some(0));
        assert_eq!(
            back.body[1].at("uris").and_then(Value::as_strings),
            Some(vec!["file:///home/me/a%20file.png".to_string()])
        );
    }

    #[test]
    fn an_error_reply_says_what_the_other_side_called_it() {
        let complaint = Message {
            kind: ERROR,
            error: Some("org.freedesktop.DBus.Error.UnknownMethod".to_string()),
            ..Message::default()
        }
        .with(vec![Value::text("No such interface")])
        .complaint();
        assert_eq!(
            complaint,
            "org.freedesktop.DBus.Error.UnknownMethod: No such interface"
        );
    }

    #[test]
    fn a_bus_address_is_read_the_way_it_is_written() {
        assert_eq!(unescaped("/run/user/1000/bus"), "/run/user/1000/bus");
        assert_eq!(unescaped("/tmp/dbus%2dtest"), "/tmp/dbus-test");
        assert_eq!(unescaped("%"), "%");
        assert!(connect("tcp:host=localhost").is_err());
        assert!(connect("unix:path=/nowhere/at/all/bus").is_err());
    }
}
