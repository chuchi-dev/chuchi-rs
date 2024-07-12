use std::fmt::Write;
use std::path::Path;
use std::{env, fs};

fn main() {
	let out_dir = env::var("OUT_DIR").unwrap();

	generate_mime(&out_dir);
}

// (const, canHaveCharsetUtf8, [types..], [extensions..])
const MIMES: &[(&str, bool, &[&str], &[&str])] = &[
	// text
	("TEXT", true, &["text/plain"], &["txt"]),
	("HTML", true, &["text/html"], &["html"]),
	("JS", true, &["application/javascript"], &["js", "mjs", "cjs"]),
	("CSS", true, &["text/css"], &["css"]),
	("JSON", true, &["application/json"], &["json"]),
	("CSV", true, &["text/csv"], &["csv"]),
	("DOC", false, &["application/msword"], &["doc"]),
	("DOCX", false,
		&["application/vnd.openxmlformats-officedocument.wordprocessingml.document"],
		&["docx"]
	),
	("PDF", false, &["application/pdf"], &["pdf"]),
	("PHP", true, &["application/php"], &["php"]),
	("RTF", false, &["application/rtf"], &["rtf"]),
	("SH", false, &["application/x-sh"], &["sh"]),
	("VSD", false, &["application/vnd.visio"], &["vsd"]),
	("XML", true, &["text/xml"], &["xml"]),

	// imgs
	("JPG", false, &["image/jpeg"], &["jpg"]),
	("PNG", false, &["image/png"], &["png"]),
	("GIF", false, &["image/gif"], &["gif"]),
	("SVG", false, &["image/svg+xml"], &["svg"]),
	("ICO", false, &["image/vnd.microsoft.icon"], &["ico"]),
	("TIFF", false, &["image/tiff"], &["tiff"]),
	("WBP", false, &["image/webp"], &["webp"]),

	// fonts
	("EOT", false, &["application/vnd.ms-fontobject"], &["eot"]),
	("TTF", false, &["font/ttf"], &["ttf"]),
	("WOFF", false, &["font/woff"], &["woff"]),
	("WOOF2", false, &["font/woff2"], &["woff2"]),

	// video
	("AVI", false, &["video/x-msvideo"], &["avi"]),
	("OGV", false, &["video/ogg"], &["ogv"]),
	("WEBM", false, &["video/webm"], &["webm"]),
	("MP4", false, &["video/mp4"], &["mp4"]),

	// audio
	("AAC", false, &["audio/aac"], &["aac"]),
	("MP3", false, &["audio/mpeg"], &["mp3"]),
	("OGA", false, &["audio/ogg"], &["oga"]),
	("WAV", false, &["audio/wav"], &["wav"]),
	("WEBA", false, &["audio/webm"], &["weba"]),

	// Archives
	("RAR", false, &["application/vnd.rar"], &["rar"]),
	("TAR", false, &["application/x-tar"], &["tar"]),
	("ZIP", false, &["application/zip"], &["zip"]),
	("_7ZIP", false, &["application/x-7z-compressed"], &["7z"]),

	// Binary
	("JAR", false, &["application/java-archive"], &["jar"]),
	("BINARY", false, &["application/octet-stream"], &["bin"]),
	("WASM", false, &["application/wasm"], &["wasm"])
];

fn generate_mime(out_dir: &str) {
	let dest_path = Path::new(&out_dir).join("mime.rs");

	let mut value_enum = "\
		#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]\n\
		enum MimeValue {\n"
		.to_string();

	let mut mime_consts = String::new();
	let mut from_extension_fn =
		"\tfn from_extension(e: &str) -> Option<Self> {\n\
		\t\tmatch e {\n"
			.to_string();
	let mut from_str_fn = "\tfn from_str(s: &str) -> Option<Self> {\n\
		\t\tmatch s {\n"
		.to_string();
	let mut extension_fn = "\tfn extension(&self) -> &'static str {\n\
		\t\tmatch self {\n"
		.to_string();
	let mut as_str_fn = "\tfn as_str(&self) -> &'static str {\n\
		\t\tmatch self {\n"
		.to_string();
	let mut as_str_with_maybe_charset =
		"\tfn as_str_with_maybe_charset(&self) -> &'static str {\n\
		\t\tmatch self {\n"
			.to_string();

	for (id, utf8, types, extensions) in MIMES {
		writeln!(
			value_enum,
			"\
		\t#[allow(clippy::upper_case_acronyms)]\n\
		\t{id},"
		)
		.unwrap();

		write!(
			mime_consts,
			"\t/// File extensions: {}  \n\
			\t/// mime types: {}\n\
			\tpub const {id}: Self = Self(MimeValue::{id});\n",
			extensions.join(", "),
			types.join(", ")
		)
		.unwrap();

		for ext in *extensions {
			writeln!(from_extension_fn, "\t\t\t\"{ext}\" => Some(Self::{id}),")
				.unwrap();
		}

		for ty in *types {
			writeln!(from_str_fn, "\t\t\t\"{ty}\" => Some(Self::{id}),")
				.unwrap();

			if *utf8 {
				writeln!(
					from_str_fn,
					"\t\t\t\"{ty}; charset=utf-8\" => Some(Self::{id}),"
				)
				.unwrap();
			}
		}

		let ext = extensions.first().unwrap();
		writeln!(extension_fn, "\t\t\tSelf::{id} => \"{ext}\",").unwrap();

		let ty = types.first().unwrap();
		writeln!(as_str_fn, "\t\t\tSelf::{id} => \"{ty}\",").unwrap();

		if *utf8 {
			writeln!(
				as_str_with_maybe_charset,
				"\t\t\tSelf::{id} => \"{ty}; charset=utf-8\","
			)
			.unwrap();
		} else {
			writeln!(
				as_str_with_maybe_charset,
				"\t\t\tSelf::{id} => \"{ty}\","
			)
			.unwrap();
		}
	}

	// now end functions
	writeln!(value_enum, "}}").unwrap();

	write!(
		from_extension_fn,
		"\t\t\t_ => None\n\
		\t\t}}\n\
		\t}}\n"
	)
	.unwrap();

	write!(
		from_str_fn,
		"\t\t\t_ => None\n\
		\t\t}}\n\
		\t}}\n"
	)
	.unwrap();

	write!(
		extension_fn,
		"\t\t}}\n\
		\t}}\n"
	)
	.unwrap();

	write!(
		as_str_fn,
		"\t\t}}\n\
		\t}}\n"
	)
	.unwrap();

	write!(
		as_str_with_maybe_charset,
		"\t\t}}\n\
		\t}}\n"
	)
	.unwrap();

	// impl value enum
	let content = format!(
		"{value_enum}\n\
		impl MimeValue {{\n\
		{from_extension_fn}\n\
		{from_str_fn}\n\
		{extension_fn}\n\
		{as_str_fn}\n\
		{as_str_with_maybe_charset}\
		}}\n\n\
		impl Mime {{\n\
		{mime_consts}\
		}}"
	);

	fs::write(dest_path, content).unwrap();
}
