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

use std::io::{self, Read};

extern crate clap;
use clap::{App, Arg};

extern crate picto;
use picto::buffer;
use picto::color::{Rgba, Gradient, Blend, Limited};
use picto::processing::prelude::*;

extern crate xcb;
extern crate xcb_util as xcbu;

const ATOMS: &'static [&'static str] = &[
	"_XROOTPMAP_ID",      // hsetroot
	"_XSETROOT_ID",       // xsetroot
	"_XENU_BACKGROUND_ID" // meh
];

fn main() {
	let matches = App::new("xenu-background")
		.version(env!("CARGO_PKG_VERSION"))
		.about("Xenu is artistic, and will paint a background on your screen.")
		.arg(Arg::with_name("display")
			.short("d")
			.long("display")
			.takes_value(true)
			.help("The display to connect to."))
		.arg(Arg::with_name("solid")
			.short("s")
			.long("solid")
			.takes_value(true)
			.required_unless("gradient")
			.required_unless("PATH")
			.validator(is_color)
			.help("Make a solid color background."))
		.arg(Arg::with_name("gradient")
			.short("g")
			.long("gradient")
			.multiple(true)
			.takes_value(true)
			.required_unless("PATH")
			.required_unless("solid")
			.validator(is_color)
			.help("Define a gradient as background."))
		.arg(Arg::with_name("horizontal")
			.short("-H")
			.long("horizontal")
			.requires("gradient")
			.help("Make an horizontal gradient (this is the default)."))
		.arg(Arg::with_name("vertical")
			.short("V")
			.long("vertical")
			.requires("gradient")
			.help("Make a vertical gradient."))
		.arg(Arg::with_name("PATH")
			.index(1)
			.required_unless("solid")
			.required_unless("gradient")
			.help("Path to the image to use."))
		.arg(Arg::with_name("center")
			.short("c")
			.long("center")
			.requires("PATH")
			.conflicts_with("position")
			.conflicts_with("tile")
			.help("Center the image."))
		.arg(Arg::with_name("position")
			.short("p")
			.long("position")
			.requires("PATH")
			.conflicts_with("center")
			.conflicts_with("tile")
			.takes_value(true)
			.validator(is_offset)
			.help("Set the image at the given position."))
		.arg(Arg::with_name("tile")
			.short("t")
			.long("tile")
			.requires("PATH")
			.conflicts_with("position")
			.conflicts_with("center")
			.takes_value(true)
			.validator(is_offset)
			.help("Tile the image over the screen."))
		.arg(Arg::with_name("fit")
			.short("F")
			.long("fit")
			.requires("PATH")
			.help("Fit thet image to the screen, maintaining the ratio."))
		.arg(Arg::with_name("resize")
			.short("R")
			.long("resize")
			.requires("PATH")
			.takes_value(true)
			.validator(is_size)
			.help("Resize the image to the given dimensions."))
		.arg(Arg::with_name("scale")
			.short("S")
			.long("scale")
			.requires("PATH")
			.takes_value(true)
			.validator(is_scale)
			.help("Scale the image by a factor."))
		.arg(Arg::with_name("crop")
			.short("C")
			.long("crop")
			.requires("PATH")
			.takes_value(true)
			.validator(is_area)
			.help("Crop the image before applying."))
		.arg(Arg::with_name("opacity")
			.short("O")
			.long("opacity")
			.requires("PATH")
			.takes_value(true)
			.validator(is_opacity)
			.help("Set the opacity of the background."))
		.arg(Arg::with_name("flip")
			.short("f")
			.long("flip")
			.multiple(true)
			.takes_value(true)
			.help("Flip the loaded image (takes `vertical` and `horizontal`."))
		.get_matches();

	let (connection, screen) = xcb::Connection::connect(matches.value_of("display")).unwrap();
	let setup   = connection.get_setup();
	let screen  = setup.roots().nth(screen as usize).unwrap();
	let width   = screen.width_in_pixels() as u32;
	let height  = screen.height_in_pixels() as u32;
	let opacity = to_opacity(matches.value_of("opacity").unwrap_or("1.0"));

	let mut source: buffer::Rgb = if let Some(colors) = matches.values_of("gradient") {
		buffer::Rgb::from_gradient(width, height,
			if matches.is_present("vertical") {
				picto::Orientation::Vertical
			}
			else {
				picto::Orientation::Horizontal
			},

			Gradient::new(colors.map(|c| to_color(c).into())))
	}
	else if let Some(solid) = matches.value_of("solid") {
		buffer::Rgb::from_pixel(width, height, &to_color(solid))
	}
	else {
		buffer::Rgb::new(width, height)
	};

	if let Some(path) = matches.value_of("PATH") {
		let mut image: buffer::Rgba = if path != "-" {
			picto::read::from_path(path)
		}
		else {
			let mut buffer = Vec::new();
			io::stdin().read_to_end(&mut buffer).unwrap();

			picto::read::from_memory(&buffer)
		}.expect("failed to open image");

		if matches.is_present("crop") {
			let area = to_area(matches.value_of("crop").unwrap());

			image = image.view(picto::Area::new()
				.x(area.x).y(area.y)
				.width(area.width).height(area.height)).convert();
		}

		if matches.is_present("fit") {
			image = if image.width() < width && image.height() < height {
				image.scale_to::<scaler::Lanczos3>(width, height)
			}
			else {
				image.scale_to::<scaler::Cubic>(width, height)
			};
		}
		else if matches.is_present("resize") {
			let (width, height) = to_size(matches.value_of("resize").unwrap());

			image = if image.width() < width && image.height() < height {
				image.resize::<scaler::Lanczos3>(width, height)
			}
			else {
				image.resize::<scaler::Cubic>(width, height)
			};
		}
		else if matches.is_present("scale") {
			let by = to_scale(matches.value_of("scale").unwrap());

			image = if by < 1.0 {
				image.scale_by::<scaler::Lanczos3>(by)
			}
			else {
				image.scale_by::<scaler::Cubic>(by)
			};
		}

		if matches.is_present("center") {
			let x_diff = (width - image.width()) / 2;
			let y_diff = (height - image.height()) / 2;

			for (x, y, mut px) in source.pixels_mut() {
				if x >= x_diff && x < width - x_diff && x - x_diff < image.width() &&
				   y >= y_diff && y < height - y_diff && y - y_diff < image.height()
				{
					let i = px.get();
					let o = with_opacity(&image.get(x - x_diff, y - y_diff), opacity);

					px.set(&o.over(i.into()));
				}
			}
		}
		else if matches.is_present("position") {
			let (xo, yo) = to_offset(matches.value_of("position").unwrap());

			for (x, y, mut px) in source.pixels_mut() {
				let x = x as i64;
				let y = y as i64;

				if x >= xo && x - xo < image.width() as i64 &&
				   y >= yo && y - yo < image.height() as i64
				{
					let i = px.get();
					let o = with_opacity(&image.get((x - xo) as u32, (y - yo) as u32), opacity);

					px.set(&o.over(i.into()));
				}
			}
		}
		else if matches.is_present("tile") {
			let (width, height) = (image.width() as i64, image.height() as i64);
			let (xo, yo)        = to_offset(matches.value_of("tile").unwrap_or("0/0"));

			#[inline(always)]
			fn clamp(i: i64, s: i64) -> u32 {
				if i < 0 {
					clamp(s + i, s)
				}
				else {
					(i % s) as u32
				}
			}

			for (x, y, mut px) in source.pixels_mut() {
				let x = x as i64;
				let y = y as i64;

				let i = px.get();
				let o = with_opacity(&image.get(clamp(x + xo, width), clamp(y + yo, height)), opacity);

				px.set(&o.over(i.into()));
			}
		}
		else {
			image = if image.width() < width && image.height() < height {
				image.resize::<scaler::Lanczos3>(width, height)
			}
			else {
				image.resize::<scaler::Cubic>(width, height)
			};

			for (x, y, mut px) in source.pixels_mut() {
				let i = px.get();
				let o = with_opacity(&image.get(x, y), opacity);

				px.set(&o.over(i.into()));
			}
		}
	}

	// Flip the image as requested.
	if let Some(flip) = matches.values_of("flip") {
		for side in flip {
			match side.to_lowercase().as_ref() {
				"vertically" | "vertical" | "vert" | "v" =>
					source.flip(flip::Vertically),

				"horizontally" | "horizontal" | "horiz" | "h" =>
					source.flip(flip::Horizontally),

				_ => ()
			}
		}
	}

	clean(&connection, &screen);
	set(&connection, &screen, source);

	connection.flush();
}

/// Set the background for the screen from the given image.
fn set(c: &xcb::Connection, screen: &xcb::Screen, source: buffer::Rgb) {
	// Create a shared image to store the pixels.
	let mut image = xcbu::image::shm::create(c, screen.root_depth(),
		screen.width_in_pixels(), screen.height_in_pixels())
			.expect("could not create image");

	// Fill in the pixels from the source image.
	for (x, y, px) in source.pixels() {
		let px = px.get();

		image.put(x, y,
			(((px.red   * 255.0) as u32) << 16) |
			(((px.green * 255.0) as u32) <<  8) |
			(((px.blue  * 255.0) as u32)));
	}

	// Create the pixmap that will store the background.
	let pixmap = c.generate_id();
	xcb::create_pixmap_checked(c, screen.root_depth(), pixmap, screen.root(),
		screen.width_in_pixels(), screen.height_in_pixels())
			.request_check().expect("could not create pixmap");

	// Create the useless graphics context.
	let context = c.generate_id();
	xcb::create_gc_checked(c, context, screen.root(), &[])
		.request_check().expect("could not create context");

	// Push the shared image into the pixmap.
	xcbu::image::shm::put(c, pixmap, context, &image, 0, 0, 0, 0,
		screen.width_in_pixels(), screen.height_in_pixels(), false)
			.expect("could not draw image");

	// Set the created pixmap as root background.
	xcb::change_window_attributes_checked(c, screen.root(), &[
		(xcb::CW_BACK_PIXMAP, pixmap)])
			.request_check().expect("could not change attributes");

	// Clear the root window so the background image is refreshed.
	xcb::clear_area_checked(c, true, screen.root(), 0, 0,
		screen.width_in_pixels(), screen.height_in_pixels())
			.request_check().expect("could not clear root window");

	// Set background ID.
	for atom in ATOMS {
		xcb::change_property(c, xcb::PROP_MODE_REPLACE as u8, screen.root(),
			xcb::intern_atom(c, false, atom).get_reply().expect("failed to intern atom").atom(),
			xcb::ATOM_PIXMAP, 32, &[pixmap]);
	}
}

/// Clean up any previously created resources.
fn clean(c: &xcb::Connection, screen: &xcb::Screen) {
	// Fetch any previously set pixmap IDs.
	let ids = ATOMS.iter().map(|atom| {
		let reply = xcb::get_property(c, false, screen.root(),
			xcb::intern_atom(c, false, atom).get_reply().expect("failed to intern atom").atom(),
			xcb::ATOM_PIXMAP, 0, 1).get_reply();

		match reply {
			Ok(ref reply) if reply.type_() == xcb::ATOM_PIXMAP =>
				Some(reply.value()[0]),

			_ =>
				None
		}
	}).collect::<Vec<Option<xcb::Pixmap>>>();

	// If all are set and equal, kill it with fire.
	if ids.iter().all(Option::is_some) && ids.iter().all(|id| id == ids.first().unwrap()) {
		xcb::kill_client(c, ids.first().unwrap().unwrap());
	}

	// Kill any temporary resources.
	xcb::kill_client(c, xcb::KILL_ALL_TEMPORARY);
	xcb::set_close_down_mode(c, xcb::CLOSE_DOWN_RETAIN_TEMPORARY as u8);
}

fn is_color(arg: String) -> Result<(), String> {
	if arg.starts_with('#') {
		if arg.len() == 4 || arg.len() == 7 {
			if arg.chars().skip(1).all(|c| c.is_digit(16)) {
				return Ok(());
			}
		}
	}

	Err("invalid color".into())
}

fn to_color(arg: &str) -> Rgba {
	let (r, g, b) = if arg.len() == 4 {
		(u8::from_str_radix(&arg[1..2], 16).unwrap() * 0x11,
		 u8::from_str_radix(&arg[2..3], 16).unwrap() * 0x11,
		 u8::from_str_radix(&arg[3..4], 16).unwrap() * 0x11)
	}
	else if arg.len() == 7 {
		(u8::from_str_radix(&arg[1..3], 16).unwrap() * 0x11,
		 u8::from_str_radix(&arg[3..5], 16).unwrap() * 0x11,
		 u8::from_str_radix(&arg[5..7], 16).unwrap() * 0x11)
	}
	else {
		unreachable!()
	};

	Rgba::new_u8(r, g, b, 255)
}

fn is_offset(arg: String) -> Result<(), String> {
	let parts = arg.split(':').collect::<Vec<_>>();

	if parts.len() <= 2 {
		if parts.iter().all(|s| s.chars().all(|c| c.is_digit(10) || c == '-')) {
			return Ok(());
		}
	}

	Err("offsets must be in the X:Y syntax".into())
}

fn to_offset(arg: &str) -> (i64, i64) {
	let mut parts = arg.split(':');

	(parts.next().unwrap_or("0").parse().unwrap(),
   parts.next().unwrap_or("0").parse().unwrap())
}

fn is_size(arg: String) -> Result<(), String> {
	let parts = arg.split(':').collect::<Vec<_>>();

	if parts.len() == 2 {
		if parts.iter().all(|s| s.chars().all(|c| c.is_digit(10) || c == '-')) {
			return Ok(());
		}
	}

	Err("sizes must be in the W:H syntax".into())
}

fn to_size(arg: &str) -> (u32, u32) {
	let mut parts = arg.split(':');

	(parts.next().unwrap_or("0").parse().unwrap(),
   parts.next().unwrap_or("0").parse().unwrap())
}

fn is_scale(arg: String) -> Result<(), String> {
	if arg.parse::<f32>().is_ok() {
		return Ok(());
	}

	Err("scale must be a number".into())
}

fn to_scale(arg: &str) -> f32 {
	arg.parse().unwrap()
}

fn is_area(arg: String) -> Result<(), String> {
	let parts = arg.split(':').collect::<Vec<_>>();

	if parts.len() == 4 {
		if parts.iter().all(|s| s.chars().all(|c| c.is_digit(10))) {
			return Ok(());
		}
	}

	Err("areas must be in the X:Y:W:H syntax".into())
}

fn to_area(arg: &str) -> picto::Area {
	let mut parts = arg.split(':');

	picto::Area::from(
		parts.next().unwrap_or("0").parse().unwrap(),
		parts.next().unwrap_or("0").parse().unwrap(),
		parts.next().unwrap_or("0").parse().unwrap(),
		parts.next().unwrap_or("0").parse().unwrap())
}

fn is_opacity(arg: String) -> Result<(), String> {
	if let Ok(value) = arg.parse::<f32>() {
		if value >= 0.0 && value <= 1.0 {
			return Ok(());
		}
	}

	Err("opacity must be a number between 0.0 and 1.0".into())
}

fn to_opacity(arg: &str) -> f32 {
	arg.parse().unwrap()
}

fn with_opacity(value: &Rgba, opacity: f32) -> Rgba {
	Rgba::new(value.red, value.green, value.blue,
		value.alpha - (1.0 - opacity)).clamp()
}
