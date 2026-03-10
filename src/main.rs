use async_openai::{Client, config::OpenAIConfig};
use clap::Parser;
use serde_json::{Value, json};
use std::{
    env,
    fs::File,
    io::Read,
    path::Path,
    process::{self, ExitCode},
};

#[derive(Parser)]
#[command(author, version, about)]
struct Args {
    #[arg(short = 'p', long)]
    prompt: String,
}

#[tokio::main]
async fn main() -> Result<ExitCode, Box<dyn std::error::Error>> {
    let args = Args::parse();

    let base_url = env::var("OPENROUTER_BASE_URL")
        .unwrap_or_else(|_| "https://openrouter.ai/api/v1".to_string());

    let api_key = env::var("OPENROUTER_API_KEY").unwrap_or_else(|_| {
        eprintln!("OPENROUTER_API_KEY is not set");
        process::exit(1);
    });

    let config = OpenAIConfig::new()
        .with_api_base(base_url)
        .with_api_key(api_key);

    let client = Client::with_config(config);

    #[allow(unused_variables)]
    let response: Value = client
        .chat()
        .create_byot(json!({
            "messages": [
                {
                    "role": "user",
                    "content": args.prompt
                }
            ],
            "tools": [{
                "type": "function",
                "function": {
                    "name": "Read",
                    "description": "Read and return the contents of a file",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "file_path": {
                                "type": "string",
                                "description": "The path to the file to read"
                            }
                        },
                        "required": ["file_path"]
                    }
                }
            }],
            "model": "anthropic/claude-haiku-4.5",
        }))
        .await?;

    if let Some(tool_calls) =
        response["choices"][0]["message"]["tool_calls"][0]["function"].as_object()
    {
        match tool_calls.get("name").unwrap_or_default().as_str() {
            Some("Read") => {
                let file_name = tool_calls
                    .get("arguments")
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .strip_prefix("{\"file_path\": \"")
                    .unwrap()
                    .strip_suffix("\"}");
                {
                    Read_tool::print_file(&Path::new(&file_name.unwrap()))?;
                }
            }
            Some(&_) => todo!(),
            None => todo!(),
        }
    }

    if let Some(content) = response["choices"][0]["message"]["content"].as_str() {
        println!("{}", content);
    }

    Ok(ExitCode::from(0))
}

struct Read_tool;

impl Read_tool {
    fn print_file(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let mut f = File::open(path)?;
        let mut buffer = String::new();

        f.read_to_string(&mut buffer)?;

        println!("{}", buffer);

        Ok(())
    }
}
