// Copyleft (ↄ) meh. <meh@schizofreni.co> | http://meh.schizofreni.co
//
// This file is part of xenu.
//
// xenu is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// xenu is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with xenu.  If not, see <http://www.gnu.org/licenses/>.

use std::collections::HashSet;
use std::process::Command;
use std::os::unix::process::CommandExt;
use std::env;
use std::fs;
use std::path::PathBuf;

extern crate clap;
use clap::{App, AppSettings, Arg};

fn main() {
	let mut app = App::new("xenu")
		.version(env!("CARGO_PKG_VERSION"))
		.author("meh. <meh@schizofreni.co>")
		.about("Your X11 galactic overlord.")
		.setting(AppSettings::AllowExternalSubcommands)
		.arg(Arg::with_name("list")
			.short("l")
			.long("list")
			.help("List the available commands (be warned this will execute any command starting with `xenu-`)."));

	let matches = app.clone().get_matches();
	match matches.subcommand() {
		(name, Some(submatches)) => {
			Command::new(&format!("xenu-{}", name))
				.args(submatches.values_of("").map(|args| args.collect::<Vec<&str>>()).unwrap_or(vec![]).as_ref())
				.exec();
		}

		_ => {
			if matches.is_present("list") {
				for command in commands() {
					if let Ok(output) = Command::new(&command).arg("--help").output() {
						if let Ok(output) = String::from_utf8(output.stdout) {
							let mut lines = output.lines();
							let     name  = lines.next();
							let     about = lines.next();

							if let (Some(name), Some(about)) = (name, about) {
								if name.starts_with("xenu-") {
									let mut parts   = name.splitn(2, ' ');
									let     name    = parts.next();
									let     version = parts.next();

									if let (Some(name), Some(version)) = (name, version) {
										let name = &name[5..];

										println!("{} ({}) - {}", name, version, about);
									}
								}
							}
						}
					}
				}
			}
			else {
				app.print_help().unwrap();
				println!("");
			}
		}
	}
}

fn commands() -> HashSet<PathBuf> {
	let mut commands = HashSet::new();

	for path in env::var("PATH").unwrap().split(":") {
		if let Ok(entries) = fs::read_dir(path) {
			for entry in entries {
				if let Ok(entry) = entry {
					if let Some(name) = entry.file_name().to_str() {
						if name.starts_with("xenu-") {
							commands.insert(entry.path());
						}
					}
				}
			}
		}
	}

	commands
}
