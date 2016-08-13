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
	let matches = App::new("xenu")
		.version(env!("CARGO_PKG_VERSION"))
		.author("meh. <meh@schizofreni.co>")
		.setting(AppSettings::AllowExternalSubcommands)
		.get_matches();

	if let (name, Some(matches)) = matches.subcommand() {
		let name = format!("xenu-{}", name);
		let args = if let Some(args) = matches.values_of(&name) {
			args.collect::<Vec<&str>>()
		}
		else {
			vec![]
		};

		Command::new(&name).args(args.as_ref()).exec();
	}
}
