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

    let mut messages = vec![json!({"role": "user", "content": args.prompt})];

    #[allow(unused_variables)]
    'agent: loop {
        let response: Value = client
            .chat()
            .create_byot(json!({
                "messages": messages,
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

        messages.push(serde_json::to_value(
            response["choices"][0]["message"].clone(),
        )?);

        if let Some(tool_calls) = response["choices"][0]["message"]["tool_calls"].as_array() {
            for tool in tool_calls.iter() {
                match tool["function"].get("name").unwrap_or_default().as_str() {
                    Some("Read") => {
                        let file_name = tool["function"]
                            .get("arguments")
                            .unwrap()
                            .as_str()
                            .unwrap()
                            .strip_prefix("{\"file_path\": \"")
                            .unwrap()
                            .strip_suffix("\"}");
                        {
                            let tool_res = ReadTool::read_file(Path::new(&file_name.unwrap()))?;
                            messages.push(json!(
                                    {"role": "tool",
                                    "tool_call_id": tool
                                        .get("id")
                                        .unwrap()
                                        .as_str()
                                        .unwrap(),
                                    "content": tool_res
                                }
                            ));
                        }
                    }
                    Some(&_) => todo!(),
                    None => todo!(),
                }
            }
        } else {
            if let Some(content) = response["choices"][0]["message"]["content"].as_str() {
                println!("{}", content);
                break 'agent;
            }
        }
    }

    Ok(ExitCode::from(0))
}

struct ReadTool;

impl ReadTool {
    fn read_file(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
        let mut f = File::open(path)?;
        let mut buffer = String::new();

        f.read_to_string(&mut buffer)?;

        Ok(buffer)
    }
}
