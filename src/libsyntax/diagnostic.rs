// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use codemap::{Pos, span};
use codemap;

use std::io;
use std::uint;
use std::local_data;
use extra::term;

pub type Emitter = @fn(cmsp: Option<(@codemap::CodeMap, span)>,
                       msg: &str,
                       lvl: level);

// a handler deals with errors; certain errors
// (fatal, bug, unimpl) may cause immediate exit,
// others log errors for later reporting.
pub trait handler {
    fn fatal(@mut self, msg: &str) -> !;
    fn err(@mut self, msg: &str);
    fn bump_err_count(@mut self);
    fn err_count(@mut self) -> uint;
    fn has_errors(@mut self) -> bool;
    fn abort_if_errors(@mut self);
    fn warn(@mut self, msg: &str);
    fn note(@mut self, msg: &str);
    // used to indicate a bug in the compiler:
    fn bug(@mut self, msg: &str) -> !;
    fn unimpl(@mut self, msg: &str) -> !;
    fn emit(@mut self,
            cmsp: Option<(@codemap::CodeMap, span)>,
            msg: &str,
            lvl: level);
}

// a span-handler is like a handler but also
// accepts span information for source-location
// reporting.
pub trait span_handler {
    fn span_fatal(@mut self, sp: span, msg: &str) -> !;
    fn span_err(@mut self, sp: span, msg: &str);
    fn span_warn(@mut self, sp: span, msg: &str);
    fn span_note(@mut self, sp: span, msg: &str);
    fn span_bug(@mut self, sp: span, msg: &str) -> !;
    fn span_unimpl(@mut self, sp: span, msg: &str) -> !;
    fn handler(@mut self) -> @handler;
}

struct HandlerT {
    err_count: uint,
    emit: Emitter,
}

struct CodemapT {
    handler: @handler,
    cm: @codemap::CodeMap,
}

impl span_handler for CodemapT {
    fn span_fatal(@mut self, sp: span, msg: &str) -> ! {
        self.handler.emit(Some((self.cm, sp)), msg, fatal);
        fail!();
    }
    fn span_err(@mut self, sp: span, msg: &str) {
        self.handler.emit(Some((self.cm, sp)), msg, error);
        self.handler.bump_err_count();
    }
    fn span_warn(@mut self, sp: span, msg: &str) {
        self.handler.emit(Some((self.cm, sp)), msg, warning);
    }
    fn span_note(@mut self, sp: span, msg: &str) {
        self.handler.emit(Some((self.cm, sp)), msg, note);
    }
    fn span_bug(@mut self, sp: span, msg: &str) -> ! {
        self.span_fatal(sp, ice_msg(msg));
    }
    fn span_unimpl(@mut self, sp: span, msg: &str) -> ! {
        self.span_bug(sp, ~"unimplemented " + msg);
    }
    fn handler(@mut self) -> @handler {
        self.handler
    }
}

impl handler for HandlerT {
    fn fatal(@mut self, msg: &str) -> ! {
        (self.emit)(None, msg, fatal);
        fail!();
    }
    fn err(@mut self, msg: &str) {
        (self.emit)(None, msg, error);
        self.bump_err_count();
    }
    fn bump_err_count(@mut self) {
        self.err_count += 1u;
    }
    fn err_count(@mut self) -> uint {
        self.err_count
    }
    fn has_errors(@mut self) -> bool {
        self.err_count > 0u
    }
    fn abort_if_errors(@mut self) {
        let s;
        match self.err_count {
          0u => return,
          1u => s = ~"aborting due to previous error",
          _  => {
            s = fmt!("aborting due to %u previous errors",
                     self.err_count);
          }
        }
        self.fatal(s);
    }
    fn warn(@mut self, msg: &str) {
        (self.emit)(None, msg, warning);
    }
    fn note(@mut self, msg: &str) {
        (self.emit)(None, msg, note);
    }
    fn bug(@mut self, msg: &str) -> ! {
        self.fatal(ice_msg(msg));
    }
    fn unimpl(@mut self, msg: &str) -> ! {
        self.bug(~"unimplemented " + msg);
    }
    fn emit(@mut self,
            cmsp: Option<(@codemap::CodeMap, span)>,
            msg: &str,
            lvl: level) {
        (self.emit)(cmsp, msg, lvl);
    }
}

pub fn ice_msg(msg: &str) -> ~str {
    fmt!("internal compiler error: %s", msg)
}

pub fn mk_span_handler(handler: @handler, cm: @codemap::CodeMap)
                    -> @span_handler {
    @mut CodemapT { handler: handler, cm: cm } as @span_handler
}

pub fn mk_handler(emitter: Option<Emitter>) -> @handler {
    let emit: Emitter = match emitter {
        Some(e) => e,
        None => {
            let emit: Emitter = |cmsp, msg, t| emit(cmsp, msg, t);
            emit
        }
    };

    @mut HandlerT { err_count: 0, emit: emit } as @handler
}

#[deriving(Eq)]
pub enum level {
    fatal,
    error,
    warning,
    note,
}

fn diagnosticstr(lvl: level) -> ~str {
    match lvl {
        fatal => ~"error",
        error => ~"error",
        warning => ~"warning",
        note => ~"note"
    }
}

fn diagnosticcolor(lvl: level) -> term::color::Color {
    match lvl {
        fatal => term::color::BRIGHT_RED,
        error => term::color::BRIGHT_RED,
        warning => term::color::BRIGHT_YELLOW,
        note => term::color::BRIGHT_GREEN
    }
}

fn print_maybe_styled(msg: &str, color: term::attr::Attr) {
    #[cfg(not(stage0))]
    static tls_terminal: local_data::Key<@Option<term::Terminal>> = &local_data::Key;
    #[cfg(stage0)]
    fn tls_terminal(_: @Option<term::Terminal>) {}

    let stderr = io::stderr();

    if stderr.get_type() == io::Screen {
        let t = match local_data::get(tls_terminal, |v| v.map_consume(|&k|k)) {
            None => {
                let t = term::Terminal::new(stderr);
                let tls = @match t {
                    Ok(t) => Some(t),
                    Err(_) => None
                };
                local_data::set(tls_terminal, tls);
                &*tls
            }
            Some(tls) => &*tls
        };

        match t {
            &Some(ref term) => {
                term.attr(color);
                stderr.write_str(msg);
                term.reset();
            },
            _ => stderr.write_str(msg)
        }
    } else {
        stderr.write_str(msg);
    }
}

fn print_diagnostic(topic: &str, lvl: level, msg: &str) {
    let stderr = io::stderr();

    if !topic.is_empty() {
        stderr.write_str(fmt!("%s ", topic));
    }

    print_maybe_styled(fmt!("%s: ", diagnosticstr(lvl)),
                            term::attr::ForegroundColor(diagnosticcolor(lvl)));
    print_maybe_styled(fmt!("%s\n", msg), term::attr::Bold);
}

pub fn collect(messages: @mut ~[~str])
            -> @fn(Option<(@codemap::CodeMap, span)>, &str, level) {
    let f: @fn(Option<(@codemap::CodeMap, span)>, &str, level) =
        |_o, msg: &str, _l| { messages.push(msg.to_str()); };
    f
}

pub fn emit(cmsp: Option<(@codemap::CodeMap, span)>, msg: &str, lvl: level) {
    match cmsp {
      Some((cm, sp)) => {
        let sp = cm.adjust_span(sp);
        let ss = cm.span_to_str(sp);
        let lines = cm.span_to_lines(sp);
        print_diagnostic(ss, lvl, msg);
        highlight_lines(cm, sp, lvl, lines);
        print_macro_backtrace(cm, sp);
      }
      None => {
        print_diagnostic("", lvl, msg);
      }
    }
}

fn highlight_lines(cm: @codemap::CodeMap,
                   sp: span, lvl: level,
                   lines: @codemap::FileLines) {
    let fm = lines.file;

    // arbitrarily only print up to six lines of the error
    let max_lines = 6u;
    let mut elided = false;
    let mut display_lines = /* FIXME (#2543) */ lines.lines.clone();
    if display_lines.len() > max_lines {
        display_lines = display_lines.slice(0u, max_lines).to_owned();
        elided = true;
    }
    // Print the offending lines
    for display_lines.iter().advance |line| {
        io::stderr().write_str(fmt!("%s:%u ", fm.name, *line + 1u));
        let s = fm.get_line(*line as int) + "\n";
        io::stderr().write_str(s);
    }
    if elided {
        let last_line = display_lines[display_lines.len() - 1u];
        let s = fmt!("%s:%u ", fm.name, last_line + 1u);
        let mut indent = s.len();
        let mut out = ~"";
        while indent > 0u {
            out.push_char(' ');
            indent -= 1u;
        }
        out.push_str("...\n");
        io::stderr().write_str(out);
    }

    // FIXME (#3260)
    // If there's one line at fault we can easily point to the problem
    if lines.lines.len() == 1u {
        let lo = cm.lookup_char_pos(sp.lo);
        let mut digits = 0u;
        let mut num = (lines.lines[0] + 1u) / 10u;

        // how many digits must be indent past?
        while num > 0u { num /= 10u; digits += 1u; }

        // indent past |name:## | and the 0-offset column location
        let left = fm.name.len() + digits + lo.col.to_uint() + 3u;
        let mut s = ~"";
        // Skip is the number of characters we need to skip because they are
        // part of the 'filename:line ' part of the previous line.
        let skip = fm.name.len() + digits + 3u;
        for skip.times() {
            s.push_char(' ');
        }
        let orig = fm.get_line(lines.lines[0] as int);
        for uint::range(0u,left-skip) |pos| {
            let curChar = (orig[pos] as char);
            // Whenever a tab occurs on the previous line, we insert one on
            // the error-point-squiggly-line as well (instead of a space).
            // That way the squiggly line will usually appear in the correct
            // position.
            match curChar {
                '\t' => s.push_char('\t'),
                _ => s.push_char(' '),
            };
        }
        io::stderr().write_str(s);
        let mut s = ~"^";
        let hi = cm.lookup_char_pos(sp.hi);
        if hi.col != lo.col {
            // the ^ already takes up one space
            let num_squigglies = hi.col.to_uint()-lo.col.to_uint()-1u;
            for num_squigglies.times() {
                s.push_char('~')
            }
        }
        print_maybe_styled(s + "\n", term::attr::ForegroundColor(diagnosticcolor(lvl)));
    }
}

fn print_macro_backtrace(cm: @codemap::CodeMap, sp: span) {
    for sp.expn_info.iter().advance |ei| {
        let ss = ei.callee.span.map_default(~"", |span| cm.span_to_str(*span));
        print_diagnostic(ss, note,
                         fmt!("in expansion of %s!", ei.callee.name));
        let ss = cm.span_to_str(ei.call_site);
        print_diagnostic(ss, note, "expansion site");
        print_macro_backtrace(cm, ei.call_site);
    }
}

pub fn expect<T:Clone>(diag: @span_handler,
                       opt: Option<T>,
                       msg: &fn() -> ~str) -> T {
    match opt {
       Some(ref t) => (*t).clone(),
       None => diag.handler().bug(msg()),
    }
}
