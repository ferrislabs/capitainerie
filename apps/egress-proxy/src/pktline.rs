pub const MAX_COMMAND_SECTION: usize = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefUpdate {
    pub old: String,
    pub new: String,
    pub refname: String,
}

impl RefUpdate {
    pub fn is_deletion(&self) -> bool {
        self.new.bytes().all(|b| b == b'0')
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandSection {
    Incomplete,
    Complete(Vec<RefUpdate>),
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PktError {
    #[error("malformed pkt-line")]
    Malformed,
    #[error("push certificates are not supported")]
    PushCertificate,
    #[error("command section too large")]
    TooLarge,
    #[error("no ref update command")]
    NoCommands,
}

fn valid_oid(oid: &str) -> bool {
    matches!(oid.len(), 40 | 64) && oid.bytes().all(|b| b.is_ascii_hexdigit())
}

fn parse_command(payload: &[u8]) -> Result<Option<RefUpdate>, PktError> {
    let payload = payload.strip_suffix(b"\n").unwrap_or(payload);
    let payload = payload.split(|b| *b == 0).next().unwrap_or(payload);
    let line = std::str::from_utf8(payload).map_err(|_| PktError::Malformed)?;
    if line.starts_with("shallow ") {
        return Ok(None);
    }
    if line.starts_with("push-cert") {
        return Err(PktError::PushCertificate);
    }
    let mut parts = line.splitn(3, ' ');
    let (Some(old), Some(new), Some(refname)) = (parts.next(), parts.next(), parts.next()) else {
        return Err(PktError::Malformed);
    };
    if !valid_oid(old) || !valid_oid(new) || old.len() != new.len() || refname.is_empty() {
        return Err(PktError::Malformed);
    }
    Ok(Some(RefUpdate {
        old: old.to_owned(),
        new: new.to_owned(),
        refname: refname.to_owned(),
    }))
}

pub fn parse_commands(buf: &[u8]) -> Result<CommandSection, PktError> {
    let mut commands = Vec::new();
    let mut at = 0;
    loop {
        let Some(header) = buf.get(at..at + 4) else {
            return incomplete(buf);
        };
        let length = std::str::from_utf8(header)
            .ok()
            .and_then(|h| usize::from_str_radix(h, 16).ok())
            .filter(|_| header.iter().all(u8::is_ascii_hexdigit))
            .ok_or(PktError::Malformed)?;
        match length {
            0 => {
                return if commands.is_empty() {
                    Err(PktError::NoCommands)
                } else {
                    Ok(CommandSection::Complete(commands))
                };
            }
            1..=3 => return Err(PktError::Malformed),
            _ => {}
        }
        let Some(payload) = buf.get(at + 4..at + length) else {
            return incomplete(buf);
        };
        commands.extend(parse_command(payload)?);
        at += length;
    }
}

fn incomplete(buf: &[u8]) -> Result<CommandSection, PktError> {
    if buf.len() > MAX_COMMAND_SECTION {
        Err(PktError::TooLarge)
    } else {
        Ok(CommandSection::Incomplete)
    }
}

pub fn check_push(commands: &[RefUpdate], branch: &str) -> Result<(), &'static str> {
    let target = format!("refs/heads/{branch}");
    for command in commands {
        if command.refname != target {
            return Err("push is only allowed on the execution branch");
        }
        if command.is_deletion() {
            return Err("deleting a ref is not allowed");
        }
    }
    Ok(())
}
