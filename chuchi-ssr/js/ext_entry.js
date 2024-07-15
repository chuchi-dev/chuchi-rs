import * as location from 'ext:deno_web/12_location.js';
import * as _event from 'ext:deno_web/02_event.js';
import * as timers from 'ext:deno_web/02_timers.js';
import * as base64 from 'ext:deno_web/05_base64.js';
import * as encoding from 'ext:deno_web/08_text_encoding.js';
import * as console from 'ext:deno_console/01_console.js';
import * as compression from 'ext:deno_web/14_compression.js';
import * as performance from 'ext:deno_web/15_performance.js';
import * as crypto from 'ext:deno_crypto/00_crypto.js';
import * as url from 'ext:deno_url/00_url.js';
import * as urlPattern from 'ext:deno_url/01_urlpattern.js';
import * as streams from 'ext:deno_web/06_streams.js';
import * as fileReader from 'ext:deno_web/10_filereader.js';
import * as file from 'ext:deno_web/09_file.js';
import * as _messagePort from 'ext:deno_web/13_message_port.js';
import { DOMException as _DOMException } from 'ext:deno_web/01_dom_exception.js';
import * as _abortSignal from 'ext:deno_web/03_abort_signal.js';
import * as _imageData from 'ext:deno_web/16_image_data.js';
import * as _globalInterfaces from 'ext:deno_web/04_global_interfaces.js';
import * as webidl from 'ext:deno_webidl/00_webidl.js';

// const { ObjectDefineProperties, DateNow } = globalThis.__bootstrap.primordials;
const { DateNow } = globalThis.__bootstrap.primordials;

// function nonEnumerable(value) {
// 	return {
// 		value,
// 		writable: true,
// 		enumerable: false,
// 		configurable: true
// 	};
// }

// function writable(value) {
// 	return {
// 		value,
// 		writable: true,
// 		enumerable: true,
// 		configurable: true
// 	};
// }

globalThis.Location = location.locationConstructorDescriptor;
globalThis.location = location.locationDescriptor;

globalThis.clearInterval = timers.clearInterval;
globalThis.clearTimeout = timers.clearTimeout;
globalThis.setInterval = timers.setInterval;
globalThis.setTimeout = timers.setTimeout;

globalThis.atob = base64.atob;
globalThis.btoa = base64.btoa;

globalThis.TextDecoder = encoding.TextDecoder;
globalThis.TextEncoder = encoding.TextEncoder;
globalThis.TextDecoderStream = encoding.TextDecoderStream;
globalThis.TextEncoderStream = encoding.TextEncoderStream;

globalThis.console = new console.Console((msg, level) =>
	Deno.core.print(msg, level > 1)
);

globalThis.CompressionStream = compression.CompressionStream;
globalThis.DecompressionStream = compression.DecompressionStream;

// ObjectDefineProperties(globalThis, {
// 	Performance: nonEnumerable(performance.Performance)
// });
globalThis.Performance = performance.Performance;
globalThis.PerformanceEntry = performance.PerformanceEntry;
globalThis.PerformanceMark = performance.PerformanceMark;
globalThis.PerformanceMeasure = performance.PerformanceMeasure;
globalThis.performance = performance.performance;

globalThis.CryptoKey = crypto.CryptoKey;
globalThis.crypto = crypto.crypto;
globalThis.Crypto = crypto.Crypto;
globalThis.SubtleCrypto = crypto.SubtleCrypto;

globalThis.URL = url.URL;
globalThis.URLSearchParams = url.URLSearchParams;
globalThis.URLPattern = urlPattern.URLPattern;

globalThis.ByteLengthQueuingStrategy = streams.ByteLengthQueuingStrategy;
globalThis.CountQueuingStrategy = streams.CountQueuingStrategy;
globalThis.ReadableStream = streams.ReadableStream;
globalThis.ReadableStreamDefaultReader = streams.ReadableStreamDefaultReader;
globalThis.TransformStream = streams.TransformStream;
globalThis.WritableStream = streams.WritableStream;
globalThis.WritableStreamDefaultWriter = streams.WritableStreamDefaultWriter;
globalThis.WritableStreamDefaultController =
	streams.WritableStreamDefaultController;
globalThis.ReadableByteStreamController = streams.ReadableByteStreamController;
globalThis.ReadableStreamBYOBReader = streams.ReadableStreamBYOBReader;
globalThis.ReadableStreamBYOBRequest = streams.ReadableStreamBYOBRequest;
globalThis.ReadableStreamDefaultController =
	streams.ReadableStreamDefaultController;
globalThis.TransformStreamDefaultController =
	streams.TransformStreamDefaultController;

globalThis.FileReader = fileReader.FileReader;

globalThis.Blob = file.Blob;
globalThis.File = file.File;

globalThis[webidl.brand] = webidl.brand;

globalThis.tracing = {
	trace: Deno.core.ops.op_tracing_trace,
	debug: Deno.core.ops.op_tracing_debug,
	info: Deno.core.ops.op_tracing_info,
	warn: Deno.core.ops.op_tracing_warn,
	error: Deno.core.ops.op_tracing_error,
};

globalThis.fetch = async (addr, params = {}) => {
	let headers = params.headers ?? {};
	let method = params.method ?? 'GET';
	let body = params.body ?? '';
	method = method.toUpperCase();

	const resp = await Deno.core.ops.op_fetch({
		url: addr,
		method,
		headers,
		body,
	});

	return {
		ok: resp.status >= 200 && resp.status < 300,
		status: resp.status,
		headers: resp.headers,
		json: async () => {
			return JSON.parse(resp.body);
		},
		text: async () => {
			return resp.body;
		},
	};
};

performance.setTimeOrigin(DateNow());
