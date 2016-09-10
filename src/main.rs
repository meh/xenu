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

use std::process::Command;
use std::os::unix::process::CommandExt;

extern crate clap;
use clap::{App, AppSettings};

fn main() {
	let mut app = App::new("xenu")
		.version(env!("CARGO_PKG_VERSION"))
		.author("meh. <meh@schizofreni.co>")
		.about("Your X11 galactic overlord.")
		.setting(AppSettings::AllowExternalSubcommands);

	match app.clone().get_matches().subcommand() {
		(name, Some(matches)) => {
			Command::new(&format!("xenu-{}", name))
				.args(matches.values_of("").map(|args| args.collect::<Vec<&str>>()).unwrap_or(vec![]).as_ref())
				.exec();
		}

		_ => {
			app.print_help().unwrap();
			println!("");
		}
	}
}
