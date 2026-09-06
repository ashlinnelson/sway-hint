use iced::widget::{column, container, row, scrollable, text, text_input, Column};
use iced::{color, Font, Length, Size, Theme};

use std::collections::HashMap;
use std::process::Command;

const MODE_MAP: &[(&str, &str)] = &[
    ("Mod4", "Super"),
    ("Mod1", "Alt"),
    ("Mod2", "Mod2"),
    ("Mod3", "Mod3"),
    ("Mod5", "Mod5"),
    ("Control", "Ctrl"),
];

#[derive(Clone, Debug)]
struct Binding {
    keys: String,
    command: String,
    mode: Option<String>,
}

struct App {
    filter: String,
    bindings: Vec<Binding>,
    error: Option<String>,
}

#[derive(Debug, Clone)]
enum Message {
    FilterChanged(String),
}

impl App {
    fn new(bindings: Vec<Binding>, error: Option<String>) -> Self {
        Self {
            filter: String::new(),
            bindings,
            error,
        }
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::FilterChanged(value) => self.filter = value,
        }
    }

    fn theme(&self) -> Theme {
        Theme::custom(
            "SwayBinds",
            iced::theme::Palette {
                background: color!(0x101218),
                text: color!(0xE8EAF2),
                primary: color!(0x6366F1),
                success: color!(0x34D399),
                warning: color!(0xFBBF24),
                danger: color!(0xF87171),
            },
        )
    }

    fn view(&self) -> Element<'_, Message> {
        let body: Element<'_, Message> = if let Some(err) = &self.error {
            text(format!("Error: {err}"))
                .size(14)
                .font(Font::MONOSPACE)
                .color(color!(0xF87171))
                .into()
        } else {
            self.list().into()
        };

        container(body)
            .padding(24)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn list(&self) -> Column<'_, Message, Theme> {
        let matching: Vec<&Binding> = self
            .bindings
            .iter()
            .filter(|b| self.matches_filter(b))
            .collect();

        let mut items = column![].spacing(2);
        for binding in &matching {
            items = items.push(binding_row(binding));
        }

        if matching.is_empty() {
            items = items.push(
                container(
                    text("No matching bindings")
                        .size(14)
                        .font(Font::MONOSPACE)
                        .color(color!(0x565D70)),
                )
                .padding(8),
            );
        }

        let header = row![
            text("KEY BINDINGS")
                .size(12)
                .font(Font::MONOSPACE)
                .color(color!(0x565D70)),
            text(format!("{}/{}", matching.len(), self.bindings.len()))
                .size(12)
                .font(Font::MONOSPACE)
                .color(color!(0x565D70))
                .width(Length::Fill)
                .align_x(iced::alignment::Horizontal::Right),
        ]
        .width(Length::Fill);

        column![
            header,
            text_input("Filter bindings...", &self.filter)
                .on_input(Message::FilterChanged)
                .padding([8, 12])
                .font(Font::MONOSPACE),
            scrollable(items).height(Length::Fill),
        ]
        .spacing(10)
    }

    fn matches_filter(&self, binding: &Binding) -> bool {
        let needle = self.filter.trim().to_lowercase();
        if needle.is_empty() {
            return true;
        }
        binding.keys.to_lowercase().contains(&needle)
            || binding.command.to_lowercase().contains(&needle)
            || binding
                .mode
                .as_deref()
                .unwrap_or("")
                .to_lowercase()
                .contains(&needle)
    }
}

type Element<'a, Message> = iced::Element<'a, Message, Theme>;

fn binding_row(binding: &Binding) -> container::Container<'_, Message, Theme> {
    let mut items = row![
        key_chip(&binding.keys),
        text(&binding.command)
            .size(13)
            .font(Font::MONOSPACE)
            .color(color!(0xC7CBDD))
            .width(Length::Fill),
    ]
    .spacing(12)
    .align_y(iced::alignment::Vertical::Center);

    if let Some(mode) = &binding.mode {
        items = items.push(container(text(mode).size(10).font(Font::MONOSPACE))
            .padding([2, 8])
            .style(mode_style));
    }

    container(items).padding([8, 6]).style(row_style).width(Length::Fill)
}

fn main() -> iced::Result {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.iter().any(|a| a == "-h" || a == "--help") {
        eprintln!("usage: swaybind [-c <config path>] [--dump]\nwithout -c, reads swaymsg -t get_config");
        return Ok(());
    }

    let dump = args.iter().any(|a| a == "--dump");

    let (bindings, error) = match load_config(&args) {
        Ok(src) => (parse_config(&src), None),
        Err(err) => (Vec::new(), Some(err)),
    };

    if dump {
        if let Some(err) = &error {
            eprintln!("{err}");
        }
        for b in &bindings {
            match &b.mode {
                Some(mode) => println!("[{}] {:<24} {}", mode, b.keys, b.command),
                None => println!("{:<24} {}", b.keys, b.command),
            }
        }
        println!("total: {}", bindings.len());
        return Ok(());
    }

    iced::application(
        move || App::new(bindings.clone(), error.clone()),
        App::update,
        App::view,
    )
    .theme(App::theme)
    .title("Sway Key Bindings")
    .window_size(Size::new(760.0, 640.0))
    .run()
}

fn load_config(args: &[String]) -> Result<String, String> {
    let mut path: Option<String> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-c" | "--config" => {
                i += 1;
                if i >= args.len() {
                    return Err("missing path after -c/--config".into());
                }
                path = Some(args[i].clone());
            }
            "-h" | "--help" | "--dump" => {}
            other => return Err(format!("unknown argument: {other}")),
        }
        i += 1;
    }

    match path {
        Some(p) => std::fs::read_to_string(&p).map_err(|e| format!("failed to read {p}: {e}")),
        None => {
            let output = Command::new("swaymsg")
                .args(["-t", "get_config"])
                .output()
                .map_err(|e| format!("failed to run swaymsg (is sway running?): {e}"))?;
            if !output.status.success() {
                return Err(format!("swaymsg exited with {}", output.status));
            }
            let json: serde_json::Value = serde_json::from_slice(&output.stdout)
                .map_err(|e| format!("failed to parse swaymsg output: {e}"))?;
            json.get("config")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .ok_or_else(|| "swaymsg output missing \"config\" field".to_string())
        }
    }
}

fn parse_config(src: &str) -> Vec<Binding> {
    let mut vars: HashMap<String, String> = HashMap::new();
    let mut modes: Vec<String> = Vec::new();
    let mut bindings = Vec::new();

    for raw in src.lines() {
        let line = strip_comment(raw).trim();
        if line.is_empty() {
            continue;
        }

        if line == "}" {
            modes.pop();
            continue;
        }

        let (kw, rest) = first_token_and_rest(line);

        match kw {
            "set" => {
                let (name, value) = first_token_and_rest(rest);
                if !name.is_empty() {
                    vars.insert(name.to_string(), value.to_string());
                }
            }
            "mode" if rest.contains('{') => {
                if let Some(name) = quoted_token(rest) {
                    modes.push(name.to_string());
                }
            }
            "bindsym" => {
                if let Some(binding) = parse_bindsym(rest, &vars, modes.last()) {
                    bindings.push(binding);
                }
            }
            _ => {}
        }
    }

    bindings
}

fn parse_bindsym(
    rest: &str,
    vars: &HashMap<String, String>,
    mode: Option<&String>,
) -> Option<Binding> {
    let mut rest = rest.trim_start();

    loop {
        let (token, tail) = first_token_and_rest(rest);
        if token.starts_with("--") {
            rest = tail;
        } else {
            break;
        }
    }

    let (keys, command) = first_token_and_rest(rest);
    if keys.is_empty() || command.is_empty() {
        return None;
    }

    let keys = display_keys(&resolve_keys(keys, vars));

    Some(Binding {
        keys,
        command: substitute_vars(command, vars),
        mode: mode.cloned(),
    })
}

fn strip_comment(line: &str) -> &str {
    match line.find('#') {
        Some(idx) => &line[..idx],
        None => line,
    }
}

fn first_token_and_rest(s: &str) -> (&str, &str) {
    let s = s.trim_start();
    match s.find(char::is_whitespace) {
        Some(idx) => (&s[..idx], s[idx..].trim_start()),
        None => (s, ""),
    }
}

fn quoted_token(s: &str) -> Option<&str> {
    let start = s.find('"')?;
    let end = s[start + 1..].find('"')? + start + 1;
    Some(&s[start + 1..end])
}

fn resolve_keys(s: &str, vars: &HashMap<String, String>) -> String {
    s.split('+')
        .map(|part| {
            if part.starts_with('$') {
                let mut value = vars.get(part).cloned().unwrap_or_else(|| part.to_string());
                let mut guard = 0;
                while value.starts_with('$') && vars.contains_key(&value) && guard < 16 {
                    value = vars.get(&value).cloned().unwrap_or(value);
                    guard += 1;
                }
                value
            } else {
                part.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("+")
}

fn display_keys(keys: &str) -> String {
    keys.split('+')
        .map(|part| {
            MODE_MAP
                .iter()
                .find(|(from, _)| *from == part)
                .map(|(_, to)| *to)
                .unwrap_or(part)
        })
        .collect::<Vec<_>>()
        .join("+")
}

fn substitute_vars(s: &str, vars: &HashMap<String, String>) -> String {
    let bytes = s.as_bytes();
    let mut out = String::new();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'$' {
            let mut name = String::new();
            let mut j = i + 1;
            while j < bytes.len() && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_') {
                name.push(bytes[j] as char);
                j += 1;
            }
            if !name.is_empty() {
                let key = format!("${name}");
                if let Some(value) = vars.get(&key) {
                    out.push_str(value);
                    i = j;
                    continue;
                }
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }

    out
}

fn key_chip(keys: &str) -> container::Container<'_, Message, Theme> {
    container(text(keys).size(12).font(Font::MONOSPACE))
        .padding([4, 10])
        .style(chip_style)
}

fn chip_style(theme: &Theme) -> container::Style {
    use iced::border;

    let palette = theme.extended_palette();

    container::Style {
        background: Some(palette.primary.base.color.into()),
        text_color: Some(palette.primary.base.text),
        border: border::rounded(6),
        ..Default::default()
    }
}

fn row_style(theme: &Theme) -> container::Style {
    use iced::border;

    let palette = theme.extended_palette();

    container::Style {
        background: Some(palette.background.weak.color.into()),
        border: border::rounded(8).width(1.0).color(palette.background.strong.color),
        ..Default::default()
    }
}

fn mode_style(theme: &Theme) -> container::Style {
    use iced::border;

    let palette = theme.extended_palette();

    container::Style {
        background: Some(palette.background.strong.color.into()),
        text_color: Some(palette.primary.strong.text),
        border: border::rounded(999),
        ..Default::default()
    }
}