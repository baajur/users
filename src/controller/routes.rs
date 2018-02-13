use regex::Regex;
use models::UserId;

/// List of all routes with params for the app
#[derive(Clone, Debug, PartialEq)]
pub enum Route {
    Healthcheck,
    Users,
    User(UserId),
    Current,
    JWTEmail,
    JWTGoogle,
    JWTFacebook,
}

/// RouteParser class maps regex to type-safe list of routes, defined by `enum Route`
pub struct RouteParser {
    regex_and_converters: Vec<(Regex, Box<ParamsConverter>)>,
}

type ParamsConverter = Fn(Vec<&str>) -> Option<Route>;

impl RouteParser {
    /// Creates new Router
    /// #Examples
    ///
    /// ```
    /// use users_lib::controller::routes::RouteParser;
    ///
    /// let router = RouteParser::new();
    /// ```
    pub fn new() -> Self {
        Self {
            regex_and_converters: Vec::new(),
        }
    }

    /// Adds mapping between regex and route
    /// #Examples
    ///
    /// ```
    /// use users_lib::controller::routes::{RouteParser, Route};
    ///
    /// let mut router = RouteParser::new();
    /// router.add_route(r"^/users$", Route::Users);
    /// ```
    pub fn add_route(&mut self, regex_pattern: &str, route: Route) -> &Self {
        self.add_route_with_params(regex_pattern, move |_| Some(route.clone()));
        self
    }

    /// Adds mapping between regex and route with params
    /// converter is a function with argument being a set of regex matches (strings) for route params in regex
    /// this is needed if you want to convert params from strings to int or some other types
    ///
    /// #Examples
    ///
    /// ```
    /// use users_lib::controller::routes::{RouteParser, Route};
    ///
    /// let mut router = RouteParser::new();
    /// router.add_route_with_params(r"^/users/(\d+)$", |params| {
    ///     params.get(0)
    ///        .and_then(|string_id| string_id.parse::<i32>().ok())
    ///        .map(|user_id| Route::User(user_id))
    /// });
    /// ```
    pub fn add_route_with_params<F>(&mut self, regex_pattern: &str, converter: F) -> &Self
    where
        F: Fn(Vec<&str>) -> Option<Route> + 'static,
    {
        let regex = Regex::new(regex_pattern).unwrap();
        self.regex_and_converters.push((regex, Box::new(converter)));
        self
    }

    /// Tests string router for matches
    /// Returns Some(route) if there's a match
    /// #Examples
    ///
    /// ```
    /// use users_lib::controller::routes::*;
    ///
    /// let mut router = RouteParser::new();
    /// router.add_route(r"^/users$", Route::Users);
    /// let route = router.test("/users").unwrap();
    /// assert_eq!(route, Route::Users);
    /// ```
    pub fn test(&self, route: &str) -> Option<Route> {
        self.regex_and_converters
            .iter()
            .fold(None, |acc, ref regex_and_converter| {
                if acc.is_some() {
                    return acc;
                }
                RouteParser::get_matches(&regex_and_converter.0, route).and_then(|params| regex_and_converter.1(params))
            })
    }

    fn get_matches<'a>(regex: &Regex, string: &'a str) -> Option<Vec<&'a str>> {
        regex.captures(string).and_then(|captures| {
            captures
                .iter()
                .skip(1)
                .fold(Some(Vec::<&str>::new()), |mut maybe_acc, maybe_match| {
                    if let Some(ref mut acc) = maybe_acc {
                        if let Some(mtch) = maybe_match {
                            acc.push(mtch.as_str());
                        }
                    }
                    maybe_acc
                })
        })
    }
}

pub fn create_route_parser() -> RouteParser {
    let mut router = RouteParser::new();

    // Healthcheck
    router.add_route(r"^/healthcheck$", Route::Healthcheck);

    // Users Routes
    router.add_route(r"^/users$", Route::Users);

    // Users Routes
    router.add_route(r"^/users/current$", Route::Current);

    // JWT email route
    router.add_route(r"^/jwt/email$", Route::JWTEmail);

    // JWT google route
    router.add_route(r"^/jwt/google$", Route::JWTGoogle);

    // JWT facebook route
    router.add_route(r"^/jwt/facebook$", Route::JWTFacebook);

    // Users/:id route
    router.add_route_with_params(r"^/users/(\d+)$", |params| {
        params
            .get(0)
            .and_then(|string_id| string_id.parse::<UserId>().ok())
            .map(|user_id| Route::User(user_id))
    });

    router
}
