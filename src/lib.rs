extern crate chrono;
extern crate clap;
extern crate reqwest;
extern crate serde_derive;
extern crate serde_json;

use chrono::{DateTime, Utc};
use reqwest::header::{qitem, Accept, Authorization, Bearer, Link, RelationType, UserAgent};
use serde_derive::Deserialize;
use std::{error, fmt, mem};

#[derive(Debug)]
pub struct Config {
    username: String,
    token: Option<String>,
}

impl Config {
    pub fn new(mut args: env::Args) -> Result<Config, &'static str> {
        ///
        // let args: Vec<String> = env::args().collect();
        // println!("{:?}", args);
        // let query = &args[1];
        // let filename = &args[2];
        ///

        println!("{:?}", args.next());
        let username = match args.next() {
            None => return Err("No username provided"),
            Some(arg) => arg,
        };

        let token = args.next();
        Ok(Config { username, token })
    }

    //why is this fcn optional??
    fn url(self) -> Option<String> {
        // format with {} 
        Some(format!(
            "https://api.github.com/users/{}/starred",              
            self.username
        ))          //if authenticated, should be a different link... https://developer.github.com/v3/activity/starring/ 'List Repos being starred' - starred by authenticated user
    }
}

impl<'a> From<clap::ArgMatches<'a>> for Config {
    fn from(matches: clap::ArgMatches) -> Self {
        Config {
            username: matches.value_of("USERNAME").unwrap().to_owned(),
            token: matches.value_of("TOKEN").map(String::from),
        }
    }
}

#[derive(Debug)]
struct ClientBuilder {
    inner: reqwest::ClientBuilder,
    headers: reqwest::header::Headers,
}
//client builder can be used to create a client with custom config
impl ClientBuilder {
    //constructs client builder
    fn new() -> ClientBuilder {

        //A map of header fields on requests and responses - new = new empty deaders map
        let mut headers = reqwest::header::Headers::new();

        //order x matter, set a header field
        //sets Accept header (indicated to do so in github API)
        headers.set(Accept(vec![qitem(
            "application/vnd.github.v3.star+json".parse().unwrap(),
        )]));       //specify api version- github requires this be done in request Accept header
        headers.set(UserAgent::new("supernova/0.1.0")); //all github api requests must invlude a valid user agent

        // returns new clientbuilder with inner and headers
        ClientBuilder {
            inner: reqwest::ClientBuilder::new(),
            headers,
        }
    }

    fn build(&mut self) -> reqwest::Result<reqwest::Client> {
        let headers = mem::replace(&mut self.headers, reqwest::header::Headers::new());
        self.inner.default_headers(headers).build()
    }

    fn set_authorization_token(&mut self, token: String) -> &mut ClientBuilder {
        self.headers.set(Authorization(Bearer { token }));      //Authorization header allows user agent to authenticate itself
        self
    }
}

#[derive(Debug, Deserialize)]
struct Star {
    starred_at: DateTime<Utc>,
    repo: Repository,
}

impl fmt::Display for Star {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.repo)
    }
}

#[derive(Debug, Deserialize)]
struct Repository {
    id: i32,
    html_url: String,
    full_name: String,
    description: Option<String>,
    stargazers_count: i32,
}

impl fmt::Display for Repository {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[{}]({})", self.full_name, self.html_url)?;

        if let Some(ref description) = self.description {
            write!(f, " - {}", description)?;
        }

        Ok(())
    }
}

pub fn collect_stars(config: Config) -> Result<(), Box<dyn error::Error>> {
    //new client builder (use to create a client with custom config)
    let mut builder = ClientBuilder::new();

    // match config.token to token; if no token, do nothing. if token, call fcn to set authorization header
    if let Some(ref token) = config.token {
        builder.set_authorization_token(token.to_owned());
    }

    //returns Client
    let client = builder.build()?;      //is there a point in this question mark if we never check client? https://m4rw3r.github.io/rust-questionmark-operator and if client never returns err...

    let mut stars: Vec<Star> = Vec::new();

    let mut next_link = config.url();   //returns api call url with username (why called next_link?)

    while next_link.is_some() {
        if let Some(link) = next_link {
            let mut res = client.get(&link).send()?;    //get- makes get request to URL (link) (returns RequestBuilder, a builder to construct the properties of a Request, like: add header, modify query string, etc.)
            //send constructs the Request and sends it to the target URL, returns a Response
            println!("{:?}", res.headers());
            // add if res.status().is_success() { ... } https://docs.rs/reqwest/0.8.6/reqwest/struct.Response.html
            next_link = extract_link_next(res.headers());   //get headers from the Response, and call fcn to
            println!("{:?}", next_link);
            let mut s: Vec<Star> = res.json()?; //deserialize response body as JSON
            stars.append(&mut s);
        }
    }

    // for star in stars.iter() {
    //     println!("{}", star);
    // }
    println!("Collected {} stars", stars.len());
    //println!("{} requests left until {}", stars.len());

    Ok(())
}

fn extract_link_next(headers: &reqwest::header::Headers) -> Option<String> {
    let link_headers = headers.get::<Link>();       //extract the Link header
    println!("here"); //printed 
    match link_headers {
        None => None,
        Some(links) => links
            .values()           //get Link headers LinkValues
            .iter()             // returns an iterator over the header fields 
            .find(|&val| {   
                println!("val: {:?}", val);   
                val.rel().map_or(false, |rel| {
                    rel.first()
                        .map_or(false, |rel_type| rel_type == &RelationType::Next)
                })
            })
            .and_then(|link_value| Some(link_value.link().to_owned())),
    }
}
