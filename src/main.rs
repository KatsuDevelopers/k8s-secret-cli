use clap::Parser;
use k8s_openapi::api::core::v1::{
    Secret, Namespace
};
use kube:: { 
    Client, 
    api:: {
        Api, 
        ListParams
    }
};
use inquire::Select;
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher}; 
// fn type_of<T>(_: &T) {
//     println!("{}", std::any::type_name::<T>())
// }
//
async fn get_namespace(client: Client) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let namespaces: Api<Namespace> = Api::all(client);
    return Ok(namespaces.list(&ListParams::default()).await.expect("Error getting namespace").iter()
        .map(|s| s.metadata.name.clone().unwrap()).collect::<Vec<String>>());
}


//
#[derive(Debug, Parser)]
#[command(author, version, about, long_about=None)] 
struct Args {
    #[arg(short='n', long="ns", default_value = "" )]
    namespace: String,
    #[arg(default_value = "")]
    name: String
}
//
fn check_namespace(namespace_name: &String, namespace_list: Vec<String>) -> Option<String> {
    // let matchers = SkimMatcherV2::default();
    return namespace_list
        .iter()
        .cloned()
        .find(|n| **n == *namespace_name);
}

fn fuzzy_search_namespace(namespace_name: &String, namespaces: Vec<String>) -> Vec<String> {
    let matchers = SkimMatcherV2::default();
    namespaces.iter().cloned()
        .filter(| n | matchers.fuzzy_match(n, namespace_name).unwrap_or(0) > 70)
        .collect::<Vec<String>>()
}

async fn get_specified_namespace(client: Client, namespace_name: String) -> Result<String, Box<dyn std::error::Error>> {
    let spicified_namespace: String;
    let namespaces = get_namespace(client).await.expect("Error getting all namespaces");
    let checked_namespace = check_namespace(&namespace_name, namespaces.clone());
    if namespace_name == "" {
        return Ok(Select::new("Select a namespace", namespaces).prompt().expect("error selecting namespace"));
    }
    if Some(checked_namespace).is_some() {

        return Ok(namespace_name)
    } else {
        // give them multi choice
        let namespace_options = fuzzy_search_namespace(&namespace_name, namespaces);
        spicified_namespace = Select::new("Do you mean one of these namespaces?", namespace_options).prompt().expect("error selecting namespace");
    }
    Ok(spicified_namespace)
}

async fn get_specified_secret(client: Client, secret_name: String, specified_namespace: String) -> Result<Secret, Box<dyn std::error::Error>> {
    let secrets: Api<Secret> = Api::namespaced(client, specified_namespace.as_str());
    let matcher = SkimMatcherV2::default();
    let secret_col = secrets.list(&ListParams::default())
        .await
        .expect("Error Retrieving list of secrets");
    let secret_name_col = secrets.list(&ListParams::default())
        .await
        .expect("Error Retrieving list of secrets")
        .iter()
        .map(|s| s.metadata.name.clone().unwrap()).collect::<Vec<String>>();

    if secret_name_col.contains(&secret_name) {
        return Ok(secret_col.iter()
            .cloned()
            .find(|s| s.metadata.name.as_ref() == Some(&secret_name))
            .unwrap()
            .to_owned())
    }

    let secret_options: Vec<String>;
    if secret_name == "" {
        secret_options = secret_name_col; 
    } else {
        secret_options = secret_name_col
            .into_iter()
            .filter(|n| matcher.fuzzy_match(n, secret_name.as_str()).unwrap_or(0) > 70)
            .collect();
    }

    let specified_secret = Select::new("Do you mean one of these secrets", secret_options)
        .prompt()
        .expect("Error Selecting Secret");
    Ok(
        secret_col.into_iter()
        .find(|s| s.metadata.name.as_deref() == Some(&specified_secret))
        .expect("Can't find selected Secret")
    )
}

fn choose_show_secret(secret: Secret) {
    let keys_list: Vec<String> = secret.data
        .clone()
        .unwrap()
        .keys()
        .map(|x| x.to_owned())
        .collect();
    let key_selected = Select::new("Select key for secret to show", keys_list).prompt().expect("Error selecting key");
    println!("key: {:#?}, value: {:#?}", 
        key_selected, 
        std::str::from_utf8(&secret.data.unwrap()
            .get(&key_selected)
            .ok_or("No values for key")
            .expect("Error getting value of secret")
            .0
        ).expect("Error Parsing Value or Keys"));
}
#[tokio::main]
async fn main() {
    let args = Args::parse();
    let kube_client = Client::try_default().await.expect("Error getting kube client");
    if args.name  != "" && args.namespace == "" {
        let secret = get_specified_secret(kube_client.clone(), args.name.clone(), String::from("default")).await.expect("Error Selecting Secret");
        choose_show_secret(secret)
    } else {
        let specified_namespace = get_specified_namespace(kube_client.clone(), args.namespace).await.expect("error getting specified namespace");
        let secret = get_specified_secret(kube_client.clone(), args.name, specified_namespace).await.expect("Error Selecting Secret");
        choose_show_secret(secret)
    } 
}
