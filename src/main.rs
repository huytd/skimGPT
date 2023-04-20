use std::{env, io::Write};

use async_openai::{types::{Role, CreateChatCompletionRequestArgs, ChatCompletionRequestMessageArgs}, Client};
use termimad::MadSkin;

fn sub_strings(string: &str, sub_len: usize) -> Vec<&str> {
    let mut subs = Vec::with_capacity(string.len() / sub_len);
    let mut iter = string.chars();
    let mut pos = 0;

    while pos < string.len() {
        let mut len = 0;
        for ch in iter.by_ref().take(sub_len) {
            len += ch.len_utf8();
        }
        subs.push(&string[pos..pos + len]);
        pos += len;
    }
    subs
}

#[tokio::main]
async fn main() {
    let mut stdout = std::io::stdout();
    print!("Loading...\r");
    stdout.flush().unwrap();

    let args: Vec<String> = env::args().collect();
    let url = args.get(1).expect("Please provide the article URL you want to summarize!");

    let html = reqwest::get(url).await.expect("Could not fetch the article!").text().await.expect("Could not read the article content!");
    let decorator = html2text::render::text_renderer::TrivialDecorator::new();
    let article_content = html2text::from_read_with_decorator(html.as_bytes(), 80, decorator).lines()
        .filter(|line| !line.is_empty())
        .collect::<String>();
    let chunks = sub_strings(&article_content, 3000);

    let api_key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set");
    let openai = Client::new().with_api_key(api_key);
    let mut summary = String::new();
    let total_chunks = chunks.len();
    let mut i = 0;
    for chunk in chunks {
        print!("Summarizing part {}/{}...\r", i + 1, total_chunks);
        stdout.flush().unwrap();
        i += 1;
        let request = CreateChatCompletionRequestArgs::default()
            .model("gpt-3.5-turbo")
            .stream(false)
            .messages(vec![
                      ChatCompletionRequestMessageArgs::default()
                      .role(Role::User)
                      .content(format!(r#"Given the following article\n"""\n{chunk}\n"""\nSummarize it to maximum 1000 chars."#))
                      .build()
                      .unwrap()
            ])
            .build()
            .unwrap();
	    let chat_response = openai.chat().create(request).await.expect("Could not request to OpenAI!");
	    let response_content = chat_response.choices[0].message.content.to_string();
	    summary.push_str(&response_content);
    }

    print!("Putting everything together...\r");
    stdout.flush().unwrap();
    let request = CreateChatCompletionRequestArgs::default()
        .model("gpt-3.5-turbo")
        .stream(false)
        .messages(vec![
            ChatCompletionRequestMessageArgs::default()
            .role(Role::User)
            .content(format!(r#"Given the following article\n"""\n{summary}\n"""\nDo the following things:\n1. Provide a summarize of the article.\n2. Provide five most critical questions that a reader could ask after reading this article, and what are the answer?\n3.Provide some important keywords from the article that the user can use to do further research about the topic.\nProvide the answer in the following markdown format:\n"""\n# Summary\n<summary of the article in maximum 3 paragraphs>\n# Things you need to know\n**1. Question (based on the article content)?**\n\nAnswer for question\n...\n# Go deeper\n<list of keywords>\n"""\n"#))
            .build()
            .unwrap()
        ])
        .build()
        .unwrap();
	let chat_response = openai.chat().create(request).await.expect("Could not request to OpenAI!");
	let response_content = chat_response.choices[0].message.content.to_string();

    let mut skin = MadSkin::default();
    skin.italic.add_attr(termimad::crossterm::style::Attribute::Underlined);
    skin.print_text(&response_content);
}
