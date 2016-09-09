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

extern crate clap;
use clap::{App, Arg};

extern crate image;
use image::{GenericImage, Pixel};

extern crate xcb;
extern crate xcb_util as xcbu;

fn main() {
	let matches = App::new("xenu-background")
		.version(env!("CARGO_PKG_VERSION"))
		.about("Xenu is artistic, and will paint a background on your screen.")
		.arg(Arg::with_name("display")
			.short("d")
			.long("display")
			.takes_value(true)
			.help("The display to connect to."))
		.arg(Arg::with_name("PATH")
			.index(1)
			.required(true)
			.help("Path to the image to use."))
		.get_matches();

	let path = matches.value_of("PATH").unwrap();

	let (connection, screen) = xcb::Connection::connect(matches.value_of("display")).unwrap();
	let setup  = connection.get_setup();
	let screen = setup.roots().nth(screen as usize).unwrap();

	let source = image::open(path).expect("failed to open image")
		.resize(screen.width_in_pixels() as u32, screen.height_in_pixels() as u32,
			image::FilterType::Lanczos3);

	let mut image = xcbu::image::shm::create(&connection, screen.root_depth(),
		screen.width_in_pixels(), screen.height_in_pixels()).unwrap();

	for (x, y, pixel) in source.pixels() {
		let pixel = pixel.to_rgb();
		let r = pixel[0] as u32;
		let g = pixel[1] as u32;
		let b = pixel[2] as u32;

		image.put(x, y, (r << 16) | (g << 8) | b);
	}

	let pixmap = connection.generate_id();
	xcb::create_pixmap_checked(&connection, screen.root_depth(), pixmap, screen.root(),
		screen.width_in_pixels(), screen.height_in_pixels())
			.request_check().expect("could not create pixmap");

	let context = connection.generate_id();
	xcb::create_gc_checked(&connection, context, screen.root(), &[])
		.request_check().expect("could not create context");

	xcbu::image::shm::put(&connection, pixmap, context, &image, 0, 0, 0, 0,
		screen.width_in_pixels(), screen.height_in_pixels(), false)
			.expect("could not draw image");

	xcb::change_window_attributes_checked(&connection, screen.root(), &[
		(xcb::CW_BACK_PIXMAP, pixmap)])
			.request_check().expect("could not change attributes");

	connection.flush();
}
