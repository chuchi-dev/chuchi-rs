export async function render(rawReq, opt = {}) {
	return {
		status: 200,
		fields: {
			body: '<h1>Body</h1>'
		}
	}
}