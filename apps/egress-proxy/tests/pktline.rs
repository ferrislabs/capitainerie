#![allow(clippy::unwrap_used, clippy::expect_used)]

use capitainerie_egress_proxy::pktline::{
    CommandSection, PktError, RefUpdate, check_push, parse_commands,
};

const ZERO: &str = "0000000000000000000000000000000000000000";
const OLD: &str = "1111111111111111111111111111111111111111";
const NEW: &str = "2222222222222222222222222222222222222222";
const BRANCH: &str = "capitainerie/0a1b2c3d4e5f";

fn pkt(payload: &[u8]) -> Vec<u8> {
    let mut out = format!("{:04x}", payload.len() + 4).into_bytes();
    out.extend_from_slice(payload);
    out
}

fn command(old: &str, new: &str, refname: &str, caps: bool) -> Vec<u8> {
    let mut line = format!("{old} {new} {refname}").into_bytes();
    if caps {
        line.extend_from_slice(b"\0report-status side-band-64k agent=git/2.45");
    }
    pkt(&line)
}

fn section(commands: &[Vec<u8>]) -> Vec<u8> {
    let mut out: Vec<u8> = commands.concat();
    out.extend_from_slice(b"0000PACK\x00\x00\x00\x02");
    out
}

fn complete(buf: &[u8]) -> Vec<RefUpdate> {
    match parse_commands(buf).unwrap() {
        CommandSection::Complete(commands) => commands,
        CommandSection::Incomplete => panic!("expected a complete section"),
    }
}

#[test]
fn a_push_to_the_execution_branch_is_allowed() {
    let buf = section(&[command(OLD, NEW, &format!("refs/heads/{BRANCH}"), true)]);
    let commands = complete(&buf);
    assert_eq!(commands.len(), 1);
    assert_eq!(check_push(&commands, BRANCH), Ok(()));
}

#[test]
fn a_push_to_another_branch_is_refused() {
    let buf = section(&[command(OLD, NEW, "refs/heads/main", true)]);
    assert!(check_push(&complete(&buf), BRANCH).is_err());
}

#[test]
fn a_branch_sharing_the_prefix_is_refused() {
    let buf = section(&[command(OLD, NEW, &format!("refs/heads/{BRANCH}-x"), true)]);
    assert!(check_push(&complete(&buf), BRANCH).is_err());
}

#[test]
fn a_deletion_of_the_execution_branch_is_refused() {
    let buf = section(&[command(OLD, ZERO, &format!("refs/heads/{BRANCH}"), true)]);
    let commands = complete(&buf);
    assert!(commands[0].is_deletion());
    assert!(check_push(&commands, BRANCH).is_err());
}

#[test]
fn creating_the_execution_branch_is_allowed() {
    let buf = section(&[command(ZERO, NEW, &format!("refs/heads/{BRANCH}"), true)]);
    assert_eq!(check_push(&complete(&buf), BRANCH), Ok(()));
}

#[test]
fn one_bad_command_among_several_refuses_the_push() {
    let buf = section(&[
        command(OLD, NEW, &format!("refs/heads/{BRANCH}"), true),
        command(OLD, NEW, "refs/tags/v1", false),
    ]);
    let commands = complete(&buf);
    assert_eq!(commands.len(), 2);
    assert!(check_push(&commands, BRANCH).is_err());
}

#[test]
fn several_good_commands_are_allowed() {
    let refname = format!("refs/heads/{BRANCH}");
    let buf = section(&[
        command(OLD, NEW, &refname, true),
        command(NEW, OLD, &refname, false),
    ]);
    assert_eq!(check_push(&complete(&buf), BRANCH), Ok(()));
}

#[test]
fn shallow_lines_before_the_commands_are_skipped() {
    let buf = section(&[
        pkt(format!("shallow {OLD}").as_bytes()),
        command(OLD, NEW, &format!("refs/heads/{BRANCH}"), true),
    ]);
    assert_eq!(complete(&buf).len(), 1);
}

#[test]
fn a_partial_section_asks_for_more_bytes() {
    let full = section(&[command(OLD, NEW, &format!("refs/heads/{BRANCH}"), true)]);
    for cut in [0, 2, 4, 30, full.len() - 12] {
        assert_eq!(
            parse_commands(&full[..cut]).unwrap(),
            CommandSection::Incomplete,
            "cut at {cut}"
        );
    }
}

#[test]
fn malformed_input_is_an_error() {
    assert_eq!(parse_commands(b"zzzzabcd"), Err(PktError::Malformed));
    assert_eq!(parse_commands(b"0002"), Err(PktError::Malformed));
    assert_eq!(
        parse_commands(&section(&[pkt(b"not a command")])),
        Err(PktError::Malformed)
    );
    assert_eq!(
        parse_commands(&section(&[command("abc", "def", "refs/heads/x", false)])),
        Err(PktError::Malformed)
    );
    assert_eq!(
        parse_commands(&section(&[pkt(format!("{OLD} {NEW} ").as_bytes())])),
        Err(PktError::Malformed)
    );
}

#[test]
fn a_section_without_commands_is_an_error() {
    assert_eq!(parse_commands(b"0000"), Err(PktError::NoCommands));
}

#[test]
fn push_certificates_are_refused() {
    assert_eq!(
        parse_commands(&section(&[pkt(b"push-cert\0caps")])),
        Err(PktError::PushCertificate)
    );
}

#[test]
fn an_oversized_section_without_flush_is_an_error() {
    let one = command(OLD, NEW, &format!("refs/heads/{BRANCH}"), false);
    let buf = one.repeat(2 * 1024 * 1024 / one.len());
    assert_eq!(parse_commands(&buf), Err(PktError::TooLarge));
}
