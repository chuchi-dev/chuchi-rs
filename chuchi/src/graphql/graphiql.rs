const HTML: &str = include_str!("./graphiql.html");

pub fn graphiql_source(graphql_endpoint_url: &str) -> String {
	HTML.replace("{graphql_url}", graphql_endpoint_url)
}
