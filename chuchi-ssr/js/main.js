import { render } from './server.js';

class ConcurrentCounter {
	constructor(limit) {
		this.max = limit;
		this.current = 0;

		this.listener = null;
	}

	/// returns if another up can be called
	async ready() {
		if (this.current < this.max) return;

		tracing.warn('Concurrent limit reached');

		return new Promise(resolve => {
			this.listener = resolve;
		});
	}

	up() {
		this.current += 1;
	}

	down() {
		this.current -= 1;
		if (this.listener && this.current < this.max) {
			const trigger = this.listener;
			this.listener = null;
			trigger();
		}
	}
}

const CONCURRENCY_LIMIT = 1000;

async function main() {
	const opts = Deno.core.ops.op_get_options() ?? {};
	const concurrent = new ConcurrentCounter(CONCURRENCY_LIMIT);

	while (true) {
		await concurrent.ready();
		const { id, req } = await Deno.core.ops.op_next_request();

		(async () => {
			concurrent.up();
			tracing.trace('received request');

			let resp = {};
			try {
				resp = await render(req, opts);
				if (!resp) resp = {};
				if (!resp.status) resp.status = 404;
				if (!resp.fields) resp.fields = {};
			} catch (e) {
				// todo handle this differently
				tracing.error('render error');
				resp = {
					status: 500,
					fields: {
						head: '',
						body: e.toString(),
					},
				};
			}

			Deno.core.ops.op_send_response(id, resp);
			concurrent.down();
		})();
	}
}
await main();
