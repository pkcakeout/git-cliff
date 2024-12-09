use std::path::{
	Path,
	PathBuf,
};

use crate::{
	event::{
		Event,
		EventHandler,
	},
	state::{
		Config,
		Result,
		State,
	},
};

pub mod event;
pub mod state;
pub mod ui;

use git_cliff::args::{
	Args,
	Parser,
};
use notify::{
	RecursiveMode,
	Watcher,
};
use ratatui::crossterm::event::{
	DisableMouseCapture,
	EnableMouseCapture,
};

fn main() -> Result<()> {
	// Parse command-line arguments.
	let args = Args::parse();

	// Create an application state.
	let mut state = State::new(args.clone())?;

	// Add default configuration file.
	if Path::new("cliff.toml").exists() {
		state.configs.insert(0, Config {
			file: "cliff.toml".into(),
			..Default::default()
		});
	}

	// Add the configuration file from the command-line arguments.
	if &args.config != &PathBuf::from("cliff.toml") {
		if args.config.exists() {
			state.configs.insert(0, Config {
				file: args.config.to_string_lossy().to_string(),
				..Default::default()
			});
		}
	}

	// Generate the changelog.
	state.generate_changelog()?;

	// Initialize the terminal user interface.
	let events = EventHandler::new(250);
	let mut terminal = ratatui::init();
	ratatui::crossterm::execute!(terminal.backend_mut(), EnableMouseCapture)?;

	// Watch for file changes.
	let sender = events.sender.clone();
	let mut watcher =
		notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
			match res {
				Ok(event) => {
					if event.kind.is_modify() {
						sender
							.send(Event::AutoGenerate)
							.expect("failed to send event");
					}
				}
				Err(e) => panic!("watch error: {e:?}"),
			}
		})?;

	for config in state.configs.iter() {
		let path = Path::new(&config.file);
		if path.exists() {
			watcher.watch(path, RecursiveMode::NonRecursive)?;
		}
	}

	// Start the main loop.
	while state.is_running {
		// Render the user interface.
		terminal.draw(|frame| ui::render(&mut state, frame))?;
		// Handle events.
		let event = events.next()?;
		match event {
			Event::Tick => state.tick(),
			Event::Key(key_event) => event::handle_key_events(
				key_event,
				events.sender.clone(),
				&mut state,
			)?,
			Event::Mouse(_) => {}
			Event::Resize(_, _) => {}
			Event::Generate | Event::AutoGenerate => {
				// if event == Event::AutoGenerate && !state.autoload {
				// 	continue;
				// }
				// let sender = events.sender.clone();
				// let args = state.args.clone();
				// state.is_generating = true;
				// state.args.config = PathBuf::from(
				// 	state.configs[state.list_state.selected().
				// unwrap_or_default()] 		.file
				// 		.clone(),
				// );
				// thread::spawn(move || {
				// 	let mut output = Vec::new();
				// 	sender
				// 		.send(match git_cliff::run(args, &mut output) {
				// 			Ok(()) => Event::RenderMarkdown(
				// 				String::from_utf8_lossy(&output).to_string(),
				// 			),
				// 			Err(e) => Event::Error(e.to_string()),
				// 		})
				// 		.expect("failed to send event");
				// });
			}
			Event::RenderMarkdown(_) => {
				// state.is_generating = false;
				// state.changelog = changelog;
				// state.markdown.component =
				// Some(md_tui::parser::parse_markdown( 	None,
				// 	&state.changelog,
				// 	state.markdown.area.width,
				// ));
			}
			Event::Error(e) => {
				state.error = Some(e);
			}
		}
	}

	ratatui::restore();
	ratatui::crossterm::execute!(terminal.backend_mut(), DisableMouseCapture)?;
	Ok(())
}
