use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use rand::Rng;
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Bar, BarChart, BarGroup, Block, Borders, Gauge, List, ListItem, Paragraph},
};
use std::io;

// ============================================================================
// MEMBERSHIP FUNCTIONS - Funções de Pertinência
// ============================================================================

/// Trapezoidal membership function
/// Returns the degree of membership [0.0, 1.0]
fn trapezoidal(x: f64, a: f64, b: f64, c: f64, d: f64) -> f64 {
    if x <= a || x >= d {
        0.0
    } else if x >= b && x <= c {
        1.0
    } else if x > a && x < b {
        (x - a) / (b - a)
    } else {
        (d - x) / (d - c)
    }
}

/// Triangular membership function
/// Returns the degree of membership [0.0, 1.0]
fn triangular(x: f64, a: f64, b: f64, c: f64) -> f64 {
    if x <= a || x >= c {
        0.0
    } else if x == b {
        1.0
    } else if x > a && x < b {
        (x - a) / (b - a)
    } else {
        (c - x) / (c - b)
    }
}

// ============================================================================
// FUZZY VARIABLES - Variáveis Fuzzy
// ============================================================================

#[derive(Debug, Clone)]
struct FuzzySet {
    name: String,
    membership: f64,
}

/// Temperature fuzzy sets: Cold, Mild, Hot
fn fuzzify_temperature(temp: f64) -> Vec<FuzzySet> {
    vec![
        FuzzySet {
            name: "Cold".to_string(),
            membership: trapezoidal(temp, 0.0, 0.0, 15.0, 20.0),
        },
        FuzzySet {
            name: "Mild".to_string(),
            membership: triangular(temp, 15.0, 22.5, 30.0),
        },
        FuzzySet {
            name: "Hot".to_string(),
            membership: trapezoidal(temp, 25.0, 30.0, 50.0, 50.0),
        },
    ]
}

/// Humidity fuzzy sets: Low, Medium, High
fn fuzzify_humidity(humidity: f64) -> Vec<FuzzySet> {
    vec![
        FuzzySet {
            name: "Low".to_string(),
            membership: trapezoidal(humidity, 0.0, 0.0, 30.0, 50.0),
        },
        FuzzySet {
            name: "Medium".to_string(),
            membership: triangular(humidity, 30.0, 50.0, 70.0),
        },
        FuzzySet {
            name: "High".to_string(),
            membership: trapezoidal(humidity, 50.0, 70.0, 100.0, 100.0),
        },
    ]
}

/// Fan speed fuzzy sets: Off, Low, Medium, High
fn fan_speed_sets() -> Vec<(String, f64, f64, f64)> {
    vec![
        ("Off".to_string(), 0.0, 0.0, 20.0),
        ("Low".to_string(), 0.0, 25.0, 50.0),
        ("Medium".to_string(), 25.0, 50.0, 75.0),
        ("High".to_string(), 50.0, 100.0, 100.0),
    ]
}

// ============================================================================
// FUZZY RULES - Regras Fuzzy (Mamdani Method)
// ============================================================================

#[derive(Debug, Clone)]
struct FuzzyRule {
    temp_condition: String,
    humidity_condition: String,
    fan_speed_output: String,
}

/// Define fuzzy rules for fan control
fn create_rules() -> Vec<FuzzyRule> {
    vec![
        FuzzyRule {
            temp_condition: "Cold".to_string(),
            humidity_condition: "Low".to_string(),
            fan_speed_output: "Off".to_string(),
        },
        FuzzyRule {
            temp_condition: "Cold".to_string(),
            humidity_condition: "Medium".to_string(),
            fan_speed_output: "Off".to_string(),
        },
        FuzzyRule {
            temp_condition: "Cold".to_string(),
            humidity_condition: "High".to_string(),
            fan_speed_output: "Low".to_string(),
        },
        FuzzyRule {
            temp_condition: "Mild".to_string(),
            humidity_condition: "Low".to_string(),
            fan_speed_output: "Low".to_string(),
        },
        FuzzyRule {
            temp_condition: "Mild".to_string(),
            humidity_condition: "Medium".to_string(),
            fan_speed_output: "Medium".to_string(),
        },
        FuzzyRule {
            temp_condition: "Mild".to_string(),
            humidity_condition: "High".to_string(),
            fan_speed_output: "Medium".to_string(),
        },
        FuzzyRule {
            temp_condition: "Hot".to_string(),
            humidity_condition: "Low".to_string(),
            fan_speed_output: "Medium".to_string(),
        },
        FuzzyRule {
            temp_condition: "Hot".to_string(),
            humidity_condition: "Medium".to_string(),
            fan_speed_output: "High".to_string(),
        },
        FuzzyRule {
            temp_condition: "Hot".to_string(),
            humidity_condition: "High".to_string(),
            fan_speed_output: "High".to_string(),
        },
    ]
}

// ============================================================================
// FUZZY INFERENCE ENGINE
// ============================================================================

/// Apply fuzzy rules and compute output membership for each fan speed
fn apply_rules(
    temp_sets: &[FuzzySet],
    humidity_sets: &[FuzzySet],
    rules: &[FuzzyRule],
) -> Vec<(String, f64)> {
    let mut output_memberships: Vec<(String, f64)> = Vec::new();

    for rule in rules {
        let temp_membership = temp_sets
            .iter()
            .find(|s| s.name == rule.temp_condition)
            .map(|s| s.membership)
            .unwrap_or(0.0);

        let humidity_membership = humidity_sets
            .iter()
            .find(|s| s.name == rule.humidity_condition)
            .map(|s| s.membership)
            .unwrap_or(0.0);

        let rule_strength = temp_membership.min(humidity_membership);

        if rule_strength > 0.0 {
            output_memberships.push((rule.fan_speed_output.clone(), rule_strength));
        }
    }

    output_memberships
}

// ============================================================================
// DEFUZZIFICATION - Center of Area (COA) Method
// ============================================================================

/// Defuzzify using Center of Area method
fn defuzzify(output_memberships: Vec<(String, f64)>) -> f64 {
    let fan_sets = fan_speed_sets();
    let resolution = 100;
    let mut numerator = 0.0;
    let mut denominator = 0.0;

    for i in 0..=resolution {
        let x = (i as f64 / resolution as f64) * 100.0;
        let mut max_membership: f64 = 0.0;

        for (output_name, rule_strength) in &output_memberships {
            if let Some((_, a, b, c)) = fan_sets.iter().find(|(name, _, _, _)| name == output_name)
            {
                let set_membership = triangular(x, *a, *b, *c);
                let implied_membership = rule_strength.min(set_membership);
                max_membership = max_membership.max(implied_membership);
            }
        }

        numerator += x * max_membership;
        denominator += max_membership;
    }

    if denominator == 0.0 {
        0.0
    } else {
        numerator / denominator
    }
}

// ============================================================================
// FUZZY CONTROLLER
// ============================================================================

struct FuzzyController {
    rules: Vec<FuzzyRule>,
}

impl FuzzyController {
    fn new() -> Self {
        FuzzyController {
            rules: create_rules(),
        }
    }

    fn compute(&self, temperature: f64, humidity: f64) -> f64 {
        let temp_sets = fuzzify_temperature(temperature);
        let humidity_sets = fuzzify_humidity(humidity);
        let output_memberships = apply_rules(&temp_sets, &humidity_sets, &self.rules);
        defuzzify(output_memberships)
    }
}

// ============================================================================
// APPLICATION STATE
// ============================================================================

enum InputMode {
    Menu,
    Temperature,
    Humidity,
}

struct App {
    controller: FuzzyController,
    temperature: f64,
    humidity: f64,
    fan_speed: f64,
    input_mode: InputMode,
    input_buffer: String,
    message: String,
    history: Vec<(f64, f64, f64)>,
}

impl App {
    fn new() -> Self {
        App {
            controller: FuzzyController::new(),
            temperature: 25.0,
            humidity: 50.0,
            fan_speed: 0.0,
            input_mode: InputMode::Menu,
            input_buffer: String::new(),
            message: "Welcome! Press 'r' for random, 't' to set temperature, 'h' for humidity, 'q' to quit".to_string(),
            history: Vec::new(),
        }
    }

    fn compute_fan_speed(&mut self) {
        self.fan_speed = self.controller.compute(self.temperature, self.humidity);
        self.history
            .push((self.temperature, self.humidity, self.fan_speed));
        if self.history.len() > 10 {
            self.history.remove(0);
        }
    }

    fn generate_random(&mut self) {
        let mut rng = rand::thread_rng();
        self.temperature = rng.gen_range(10.0..40.0);
        self.humidity = rng.gen_range(20.0..90.0);
        self.compute_fan_speed();
        self.message = "Generated random values!".to_string();
    }
}

// ============================================================================
// UI RENDERING
// ============================================================================

fn ui<B: ratatui::backend::Backend>(f: &mut ratatui::Frame<B>, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(12),
            Constraint::Length(3),
        ])
        .split(f.size());

    // Title
    let title = Paragraph::new("🤖 FUZZY LOGIC FAN CONTROLLER")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        );
    f.render_widget(title, chunks[0]);

    // Main content area
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);

    // Left panel: Inputs and Output
    render_left_panel(f, app, main_chunks[0]);

    // Right panel: Fuzzy memberships
    render_right_panel(f, app, main_chunks[1]);

    // History
    render_history(f, app, chunks[2]);

    // Message bar
    let msg = Paragraph::new(app.message.as_str())
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().borders(Borders::ALL).title("Status"));
    f.render_widget(msg, chunks[3]);
}

fn render_left_panel<B: ratatui::backend::Backend>(
    f: &mut ratatui::Frame<B>,
    app: &App,
    area: Rect,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Length(5),
            Constraint::Length(7),
        ])
        .split(area);

    // Temperature gauge
    let temp_color = if app.temperature < 20.0 {
        Color::Cyan
    } else if app.temperature < 30.0 {
        Color::Yellow
    } else {
        Color::Red
    };

    let temp_gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("🌡️  Temperature (°C)"),
        )
        .gauge_style(Style::default().fg(temp_color))
        .ratio(app.temperature / 50.0)
        .label(format!("{:.1}°C", app.temperature));
    f.render_widget(temp_gauge, chunks[0]);

    // Humidity gauge
    let hum_color = if app.humidity < 40.0 {
        Color::LightYellow
    } else if app.humidity < 70.0 {
        Color::LightBlue
    } else {
        Color::Blue
    };

    let hum_gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("💧 Humidity (%)"),
        )
        .gauge_style(Style::default().fg(hum_color))
        .ratio(app.humidity / 100.0)
        .label(format!("{:.1}%", app.humidity));
    f.render_widget(hum_gauge, chunks[1]);

    // Fan speed output
    let fan_color = if app.fan_speed < 15.0 {
        Color::Gray
    } else if app.fan_speed < 40.0 {
        Color::Green
    } else if app.fan_speed < 65.0 {
        Color::Yellow
    } else {
        Color::Red
    };

    let status = if app.fan_speed < 15.0 {
        "OFF"
    } else if app.fan_speed < 40.0 {
        "LOW"
    } else if app.fan_speed < 65.0 {
        "MEDIUM"
    } else {
        "HIGH"
    };

    let fan_gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title("🌀 Fan Speed"))
        .gauge_style(Style::default().fg(fan_color).add_modifier(Modifier::BOLD))
        .ratio(app.fan_speed / 100.0)
        .label(format!("{:.1}% [{}]", app.fan_speed, status));
    f.render_widget(fan_gauge, chunks[2]);
}

fn render_right_panel<B: ratatui::backend::Backend>(
    f: &mut ratatui::Frame<B>,
    app: &App,
    area: Rect,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Temperature memberships
    let temp_sets = fuzzify_temperature(app.temperature);
    let temp_bars: Vec<Bar> = temp_sets
        .iter()
        .map(|set| {
            let color = match set.name.as_str() {
                "Cold" => Color::Cyan,
                "Mild" => Color::Yellow,
                "Hot" => Color::Red,
                _ => Color::White,
            };
            Bar::default()
                .value((set.membership * 100.0) as u64)
                .style(Style::default().fg(color))
        })
        .collect();

    // let temp_labels: Vec<&str> = temp_sets.iter().map(|s| s.name.as_str()).collect();
    let temp_chart = BarChart::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Temperature Fuzzy Sets"),
        )
        .data(BarGroup::default().bars(&temp_bars))
        .bar_width(8)
        .bar_gap(2)
        .value_style(
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .label_style(Style::default().fg(Color::White));
    f.render_widget(temp_chart, chunks[0]);

    // Humidity memberships
    let hum_sets = fuzzify_humidity(app.humidity);
    let hum_bars: Vec<Bar> = hum_sets
        .iter()
        .map(|set| {
            let color = match set.name.as_str() {
                "Low" => Color::LightYellow,
                "Medium" => Color::LightBlue,
                "High" => Color::Blue,
                _ => Color::White,
            };
            Bar::default()
                .value((set.membership * 100.0) as u64)
                .style(Style::default().fg(color))
        })
        .collect();

    let hum_chart = BarChart::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Humidity Fuzzy Sets"),
        )
        .data(BarGroup::default().bars(&hum_bars))
        .bar_width(8)
        .bar_gap(2)
        .value_style(
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .label_style(Style::default().fg(Color::White));
    f.render_widget(hum_chart, chunks[1]);
}

fn render_history<B: ratatui::backend::Backend>(f: &mut ratatui::Frame<B>, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .history
        .iter()
        .rev()
        .take(5)
        .map(|(t, h, f)| {
            let status = if *f < 15.0 {
                ("OFF", Color::Gray)
            } else if *f < 40.0 {
                ("LOW", Color::Green)
            } else if *f < 65.0 {
                ("MED", Color::Yellow)
            } else {
                ("HIGH", Color::Red)
            };

            ListItem::new(Line::from(vec![
                Span::styled(format!("T: {:.1}°C ", t), Style::default().fg(Color::Cyan)),
                Span::styled(
                    format!("H: {:.1}% ", h),
                    Style::default().fg(Color::LightBlue),
                ),
                Span::styled(
                    format!("→ Fan: {:.1}% ", f),
                    Style::default().fg(Color::White),
                ),
                Span::styled(
                    format!("[{}]", status.0),
                    Style::default().fg(status.1).add_modifier(Modifier::BOLD),
                ),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("📊 Recent History"),
        )
        .style(Style::default().fg(Color::White));
    f.render_widget(list, area);
}

// ============================================================================
// EVENT HANDLING
// ============================================================================

fn handle_events(app: &mut App) -> io::Result<bool> {
    if event::poll(std::time::Duration::from_millis(100))? {
        if let Event::Key(key) = event::read()? {
            match app.input_mode {
                InputMode::Menu => match key.code {
                    KeyCode::Char('q') => return Ok(true),
                    KeyCode::Char('r') => app.generate_random(),
                    KeyCode::Char('t') => {
                        app.input_mode = InputMode::Temperature;
                        app.input_buffer.clear();
                        app.message = "Enter temperature (°C) and press Enter:".to_string();
                    }
                    KeyCode::Char('h') => {
                        app.input_mode = InputMode::Humidity;
                        app.input_buffer.clear();
                        app.message = "Enter humidity (%) and press Enter:".to_string();
                    }
                    _ => {}
                },
                InputMode::Temperature => match key.code {
                    KeyCode::Enter => {
                        if let Ok(val) = app.input_buffer.parse::<f64>() {
                            app.temperature = val.clamp(0.0, 50.0);
                            app.compute_fan_speed();
                            app.message = format!("Temperature set to {:.1}°C", app.temperature);
                        } else {
                            app.message = "Invalid input! Try again.".to_string();
                        }
                        app.input_mode = InputMode::Menu;
                        app.input_buffer.clear();
                    }
                    KeyCode::Char(c) => app.input_buffer.push(c),
                    KeyCode::Backspace => {
                        app.input_buffer.pop();
                    }
                    KeyCode::Esc => {
                        app.input_mode = InputMode::Menu;
                        app.message = "Cancelled.".to_string();
                        app.input_buffer.clear();
                    }
                    _ => {}
                },
                InputMode::Humidity => match key.code {
                    KeyCode::Enter => {
                        if let Ok(val) = app.input_buffer.parse::<f64>() {
                            app.humidity = val.clamp(0.0, 100.0);
                            app.compute_fan_speed();
                            app.message = format!("Humidity set to {:.1}%", app.humidity);
                        } else {
                            app.message = "Invalid input! Try again.".to_string();
                        }
                        app.input_mode = InputMode::Menu;
                        app.input_buffer.clear();
                    }
                    KeyCode::Char(c) => app.input_buffer.push(c),
                    KeyCode::Backspace => {
                        app.input_buffer.pop();
                    }
                    KeyCode::Esc => {
                        app.input_mode = InputMode::Menu;
                        app.message = "Cancelled.".to_string();
                        app.input_buffer.clear();
                    }
                    _ => {}
                },
            }
        }
    }
    Ok(false)
}

// ============================================================================
// MAIN FUNCTION
// ============================================================================

fn main() -> io::Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new();
    app.compute_fan_speed();

    // Main loop
    loop {
        terminal.draw(|f| ui(f, &app))?;

        if handle_events(&mut app)? {
            break;
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
