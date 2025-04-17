use reqwest;
use serde_json::{json, Value};
use std::env;
use std::collections::{VecDeque, HashMap};
use std::sync::{Arc, RwLock};
use std::fs;
use std::path::Path;
use dotenv::dotenv;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use chrono;

#[derive(Debug, Serialize, Deserialize)]
struct CharacterConfig {
    name: String,
    personality: String,
    description: String,
    traits: Vec<String>,
    interests: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct KnowledgeSources {
    self_learning_urls: Vec<String>,
    additional_context: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ConversationSettings {
    max_history: usize,
    learning_frequency: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatbotConfig {
    character: CharacterConfig,
    knowledge_sources: KnowledgeSources,
    conversation_settings: ConversationSettings,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Knowledge {
    facts: HashMap<String, String>,
    search_history: Vec<String>,
    learned_urls: Vec<String>,
    external_url_count: usize,
    cached_content: HashMap<String, String>,
}

struct Chatbot {
    config: ChatbotConfig,
    conversation_history: VecDeque<String>,
    knowledge: Arc<RwLock<Knowledge>>,
}

impl Chatbot {
    fn new(config: ChatbotConfig) -> Self {
        Chatbot {
            config,
            conversation_history: VecDeque::new(),
            knowledge: Arc::new(RwLock::new(Knowledge {
                facts: HashMap::new(),
                search_history: Vec::new(),
                learned_urls: Vec::new(),
                external_url_count: 0,
                cached_content: HashMap::new(),
            })),
        }
    }

    fn add_to_history(&mut self, message: &str) {
        if self.conversation_history.len() >= self.config.conversation_settings.max_history {
            self.conversation_history.pop_front();
        }
        self.conversation_history.push_back(message.to_string());
    }

    fn load_knowledge(&self) -> Result<(), Box<dyn std::error::Error>> {
        let knowledge_path = Path::new("data/learned_knowledge.json");
        if knowledge_path.exists() {
            let knowledge_str = fs::read_to_string(knowledge_path)?;
            let loaded_knowledge: Knowledge = serde_json::from_str(&knowledge_str)?;
            
            if let Ok(mut current_knowledge) = self.knowledge.write() {
                current_knowledge.merge(loaded_knowledge);
            }
        }
        Ok(())
    }

    fn save_knowledge(&self) -> Result<(), Box<dyn std::error::Error>> {
        if let Ok(knowledge) = self.knowledge.read() {
            let knowledge_str = serde_json::to_string_pretty(&*knowledge)?;
            
            // Ensure the data directory exists
            fs::create_dir_all("data")?;
            fs::write("data/learned_knowledge.json", knowledge_str)?;
            println!("Knowledge saved successfully");
        }
        Ok(())
    }

    async fn process_with_ai(&self, content: &str) -> Result<String, Box<dyn std::error::Error>> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;
            
        // Prepare the prompt for Gemini
        let prompt = format!(
            "You are Alisa Mikhailovna Kujou. Process this raw information about you and rewrite it in first person perspective, \
            removing any HTML, scripts, or irrelevant content. Focus only on your personality, background, relationships, and characteristics. \
            Make it natural and personal:\n\n{}", 
            content
        );

        // Call Gemini API
        let api_key = env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY not set");
        let response = client
            .post(format!(
                "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent?key={}",
                api_key
            ))
            .json(&json!({
                "contents": [{
                    "parts": [{
                        "text": prompt
                    }]
                }]
            }))
            .send()
            .await?;

        let response_json: Value = response.json().await?;
        
        // Extract the processed content
        if let Some(candidates) = response_json.get("candidates") {
            if let Some(first_candidate) = candidates[0].as_object() {
                if let Some(content) = first_candidate.get("content") {
                    if let Some(parts) = content.get("parts") {
                        if let Some(first_part) = parts[0].as_object() {
                            if let Some(text) = first_part.get("text") {
                                return Ok(text.as_str().unwrap_or("").to_string());
                            }
                        }
                    }
                }
            }
        }
        
        Ok("".to_string())
    }

    async fn learn_from_url(&self, url: &str) -> Result<(), Box<dyn std::error::Error>> {
        let knowledge = self.knowledge.write().unwrap();
        
        // Check if we've already learned from this URL
        if knowledge.learned_urls.contains(&url.to_string()) {
            println!("Already learned from URL: {}", url);
            return Ok(());
        }

        println!("Fetching content from URL: {}", url);
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()?;
        
        let response = client
            .get(url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
            .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8")
            .header("Accept-Language", "en-US,en;q=0.5")
            .send()
            .await?;
            
        if !response.status().is_success() {
            println!("Failed to fetch URL: {} (Status: {})", url, response.status());
            return Ok(());
        }
        
        println!("Successfully fetched URL, parsing content...");
        let webpage = response.text().await?;
        let document = Html::parse_document(&webpage);
        
        // Try different selectors to get content
        let selectors = [
            "p",           // Paragraphs
            "article",     // Article content
            ".content",    // Content class
            ".article",    // Article class
            "main",        // Main content
            "#content",    // Content ID
            ".wiki-content", // Wiki content
            ".mw-parser-output", // MediaWiki content
            ".character-info", // Character info
            ".profile-content", // Profile content
        ];
        
        let mut content = String::new();
        for selector_str in selectors.iter() {
            if let Ok(selector) = Selector::parse(selector_str) {
                for element in document.select(&selector) {
                    let text = element.text().collect::<Vec<_>>().join(" ");
                    if !text.trim().is_empty() {
                        content.push_str(&text);
                        content.push_str("\n\n");
                    }
                }
            }
        }
        
        if content.trim().is_empty() {
            println!("No content found at URL: {}", url);
            return Ok(());
        }
        
        // Process content with AI before saving
        println!("Processing content with AI...");
        drop(knowledge); // Release the lock while processing
        let processed_content = self.process_with_ai(&content).await?;
        let mut knowledge = self.knowledge.write().unwrap();
        
        if !processed_content.is_empty() {
            println!("Successfully processed and personalized content");
            knowledge.facts.insert(format!("personal_knowledge_{}", url), processed_content);
            knowledge.learned_urls.push(url.to_string());
            
            // Save knowledge after successful learning
            drop(knowledge); // Release the lock before saving
            self.save_knowledge()?;
        }
        
        Ok(())
    }

    async fn learn_about_self(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Starting self-learning process...");
        
        // Load existing knowledge first
        self.load_knowledge()?;
        
        // Learn from web search
        println!("Searching web for information about {}...", self.config.character.name);
        let search_query = format!("{} character personality traits background story", self.config.character.name);
        let content = self.search_web(&search_query).await?;
        
        println!("Processing search results...");
        {
            let mut knowledge = self.knowledge.write().unwrap();
            knowledge.facts.insert("self_understanding".to_string(), content);
            knowledge.search_history.push(search_query);
        }
        
        // Save after web search
        self.save_knowledge()?;
        println!("Saved initial search results");
        
        // Learn from configured URLs
        println!("Learning from configured URLs...");
        for url in &self.config.knowledge_sources.self_learning_urls {
            println!("Processing URL: {}", url);
            match self.learn_from_url(url).await {
                Ok(_) => println!("Successfully learned from URL: {}", url),
                Err(e) => println!("Error learning from URL {}: {}", url, e),
            }
            // Save after each URL
            self.save_knowledge()?;
        }
        
        println!("Self-learning process completed!");
        println!("\nI've learned about myself and I'm ready to chat!");
        println!("You can ask me questions about:");
        println!("1. My personality and traits");
        println!("2. My background and story");
        println!("3. My interests and hobbies");
        println!("4. Or anything else you'd like to know!");
        
        Ok(())
    }

    async fn search_web(&self, query: &str) -> Result<String, Box<dyn std::error::Error>> {
        println!("Executing web search for: {}", query);
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()?;
        
        // First try Google Custom Search API
        let search_url = format!(
            "https://www.googleapis.com/customsearch/v1?key={}&cx={}&q={}",
            env::var("GOOGLE_SEARCH_API_KEY").expect("GOOGLE_SEARCH_API_KEY not set"),
            env::var("GOOGLE_SEARCH_ENGINE_ID").expect("GOOGLE_SEARCH_ENGINE_ID not set"),
            query
        );

        println!("Sending request to Google Search API...");
        let response = client.get(&search_url).send().await?;
        
        if !response.status().is_success() {
            println!("Google Search API request failed: {}", response.status());
            return Ok("".to_string());
        }

        let search_results: Value = response.json().await?;
        let mut content = String::new();

        if let Some(items) = search_results.get("items") {
            let empty_vec: Vec<Value> = Vec::new();
            let array = items.as_array().unwrap_or(&empty_vec);
            println!("Found {} search results", array.len());
            
            // Process only the first result for now
            if let Some(first_item) = array.first() {
                println!("Processing first search result");
                
                if let Some(snippet) = first_item.get("snippet") {
                    if let Some(text) = snippet.as_str() {
                        content.push_str(text);
                        content.push_str("\n\n");
                    }
                }
                
                if let Some(link) = first_item.get("link") {
                    if let Some(url) = link.as_str() {
                        if !url.contains("pinterest.com") {
                            println!("Processing URL: {}", url);
                            if let Err(e) = self.learn_from_url(url).await {
                                println!("Error processing URL: {}", e);
                            }
                        }
                    }
                }
            }
        }

        // Process search content with AI
        println!("Processing search results with AI...");
        let processed_content = self.process_with_ai(&content).await?;
        
        Ok(processed_content)
    }

    fn get_context(&self) -> String {
        let mut context = format!(
            "You are a chatbot named {}. Your personality: {}. Description: {}. Traits: {}. Interests: {}.\n",
            self.config.character.name,
            self.config.character.personality,
            self.config.character.description,
            self.config.character.traits.join(", "),
            self.config.character.interests.join(", ")
        );
        
        context.push_str(&format!("Additional context: {}\n", self.config.knowledge_sources.additional_context));
        
        if let Ok(knowledge) = self.knowledge.read() {
            // Add learned facts
            for (key, value) in &knowledge.facts {
                context.push_str(&format!("\nKnowledge from {}:\n{}\n", key, value));
            }
        }
        
        if !self.conversation_history.is_empty() {
            context.push_str("\nPrevious conversation context:\n");
            for msg in &self.conversation_history {
                context.push_str(&format!("{}\n", msg));
            }
        }
        
        // Add personality guidance
        context.push_str("\nRemember to stay in character and respond according to your personality traits. ");
        context.push_str("If you're asked about something you don't know, be honest about it. ");
        context.push_str("Use your learned knowledge to provide detailed and accurate responses.\n");
        
        context
    }

    fn save_config(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config_str = serde_json::to_string_pretty(&self.config)?;
        fs::write("config/chatbot_config.json", config_str)?;
        Ok(())
    }

    async fn train_with_text(&self, text: &str) -> Result<(), Box<dyn std::error::Error>> {
        println!("Training with provided text...");
        
        // Process the text with AI to make it more personal and relevant
        let processed_content = self.process_with_ai(text).await?;
        
        if !processed_content.is_empty() {
            let mut knowledge = self.knowledge.write().unwrap();
            knowledge.facts.insert(format!("trained_knowledge_{}", chrono::Utc::now().timestamp()), processed_content);
            
            // Save the updated knowledge
            drop(knowledge);
            self.save_knowledge()?;
            println!("Successfully trained with new text!");
        }
        
        Ok(())
    }
}

impl Knowledge {
    fn merge(&mut self, other: Knowledge) {
        self.facts.extend(other.facts);
        self.search_history.extend(other.search_history);
        self.learned_urls.extend(other.learned_urls);
        self.cached_content.extend(other.cached_content);
        self.external_url_count = other.external_url_count;
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let api_key = env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY not set");
    
    // Load or create configuration
    let config_path = Path::new("config/chatbot_config.json");
    let config: ChatbotConfig = if config_path.exists() {
        let config_str = fs::read_to_string(config_path)?;
        serde_json::from_str(&config_str)?
    } else {
        // Create default config if it doesn't exist
        let default_config = ChatbotConfig {
            character: CharacterConfig {
                name: String::new(),
                personality: String::new(),
                description: String::new(),
                traits: Vec::new(),
                interests: Vec::new(),
            },
            knowledge_sources: KnowledgeSources {
                self_learning_urls: Vec::new(),
                additional_context: String::new(),
            },
            conversation_settings: ConversationSettings {
                max_history: 5,
                learning_frequency: "daily".to_string(),
            },
        };
        default_config
    };
    
    let mut chatbot = Chatbot::new(config);
    
    println!("Welcome to the Self-Learning Rust Chatbot!");
    
    // If character is not configured, ask for configuration
    if chatbot.config.character.name.is_empty() {
        println!("Let's set up your chatbot's character.");
        
        println!("\nEnter character name: ");
        let mut character_name = String::new();
        std::io::stdin().read_line(&mut character_name)?;
        chatbot.config.character.name = character_name.trim().to_string();
        
        println!("Enter character personality: ");
        let mut personality = String::new();
        std::io::stdin().read_line(&mut personality)?;
        chatbot.config.character.personality = personality.trim().to_string();
        
        println!("Enter character description: ");
        let mut description = String::new();
        std::io::stdin().read_line(&mut description)?;
        chatbot.config.character.description = description.trim().to_string();
        
        println!("Enter character traits (comma-separated): ");
        let mut traits = String::new();
        std::io::stdin().read_line(&mut traits)?;
        chatbot.config.character.traits = traits.trim().split(',').map(|s| s.trim().to_string()).collect();
        
        println!("Enter character interests (comma-separated): ");
        let mut interests = String::new();
        std::io::stdin().read_line(&mut interests)?;
        chatbot.config.character.interests = interests.trim().split(',').map(|s| s.trim().to_string()).collect();
        
        chatbot.save_config()?;
    }
    
    println!("\nChatbot initialized as: {}", chatbot.config.character.name);
    println!("Personality: {}", chatbot.config.character.personality);
    println!("\nAvailable commands:");
    println!("- Type 'exit' to quit the chat");
    println!("- Type 'learn' to make the chatbot search and learn about itself");
    println!("- Type 'train' to train the chatbot with custom text");
    println!("- Type 'add_url <url>' to add a new learning source");
    println!("- Type 'save' to save the current configuration");
    println!("- Type anything else to chat with the AI");
    
    // Initial self-learning
    println!("\nPerforming initial self-learning...");
    chatbot.learn_about_self().await?;
    
    let client = reqwest::Client::new();
    
    loop {
        println!("\nYou: ");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        
        let input = input.trim();
        
        if input.to_lowercase() == "exit" {
            println!("Goodbye!");
            break;
        }
        
        if input.to_lowercase() == "learn" {
            println!("Searching and learning about myself...");
            chatbot.learn_about_self().await?;
            continue;
        }
        
        if input.to_lowercase() == "train" {
            println!("Enter the training text (type 'END' on a new line when finished):");
            let mut training_text = String::new();
            loop {
                let mut line = String::new();
                std::io::stdin().read_line(&mut line)?;
                if line.trim() == "END" {
                    break;
                }
                training_text.push_str(&line);
            }
            chatbot.train_with_text(&training_text).await?;
            continue;
        }
        
        if input.to_lowercase() == "save" {
            chatbot.save_config()?;
            println!("Configuration saved!");
            continue;
        }
        
        if input.starts_with("add_url ") {
            let url = input[8..].trim();
            chatbot.config.knowledge_sources.self_learning_urls.push(url.to_string());
            println!("Added new learning source: {}", url);
            chatbot.save_config()?;
            continue;
        }
        
        // Add user input to history
        chatbot.add_to_history(&format!("User: {}", input));
        
        // Prepare the prompt with context
        let context = chatbot.get_context();
        let prompt = format!("{}\n\nUser: {}\n{}: ", context, input, chatbot.config.character.name);
        
        // Prepare the request to Gemini API
        let response = client
            .post(format!(
                "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent?key={}",
                api_key
            ))
            .json(&json!({
                "contents": [{
                    "parts": [{
                        "text": prompt
                    }]
                }]
            }))
            .send()
            .await?;
            
        let response_json: Value = response.json().await?;
        
        // Extract and print the response
        if let Some(candidates) = response_json.get("candidates") {
            if let Some(first_candidate) = candidates[0].as_object() {
                if let Some(content) = first_candidate.get("content") {
                    if let Some(parts) = content.get("parts") {
                        if let Some(first_part) = parts[0].as_object() {
                            if let Some(text) = first_part.get("text") {
                                let bot_response = text.as_str().unwrap_or("No response");
                                println!("\n{}: {}", chatbot.config.character.name, bot_response);
                                chatbot.add_to_history(&format!("{}: {}", chatbot.config.character.name, bot_response));
                                continue;
                            }
                        }
                    }
                }
            }
        }
        
        println!("\n{}: Sorry, I couldn't process that request.", chatbot.config.character.name);
    }
    
    Ok(())
} 