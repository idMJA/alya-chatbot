# Self-Learning Rust Chatbot

A powerful, self-learning chatbot built in Rust that uses the Gemini API for natural language processing and can learn from web sources, custom text, and user interactions.

## Creator

Created by [iaMJ](https://github.com/idMJA)

## Features

- **Character Customization**: Create and configure your own chatbot character with personality traits, interests, and background
- **Self-Learning Capabilities**:
  - Web searching for information about the character
  - Learning from provided URLs
  - Training with custom text
- **Conversation Memory**: Maintains conversation history for context-aware responses
- **Knowledge Persistence**: Saves learned information to disk for future sessions
- **Interactive Interface**: Simple command-line interface for easy interaction

## Prerequisites

- Rust (latest stable version)
- Google Gemini API Key
- Google Custom Search API Key and Search Engine ID (for web searching)

## Getting API Keys

### Google Gemini API Key

1. Go to [Google AI Studio](https://makersuite.google.com/app/apikey)
2. Sign in with your Google account
3. Click on "Get API key" or "Create API key"
4. Copy the generated API key
5. Note: The Gemini API has a free tier with generous limits

### Google Custom Search API Key and Search Engine ID

1. **Get Google Custom Search API Key**:
   - Go to [Google Cloud Console](https://console.cloud.google.com/)
   - Create a new project or select an existing one
   - Navigate to "APIs & Services" > "Library"
   - Search for "Custom Search API" and enable it
   - Go to "APIs & Services" > "Credentials"
   - Click "Create Credentials" > "API Key"
   - Copy the generated API key
   - Note: The Custom Search API has a free tier of 100 queries per day

2. **Get Search Engine ID**:
   - Go to [Google Programmable Search Engine](https://programmablesearchengine.google.com/about/)
   - Click "Create a search engine"
   - Enter the sites you want to search (or use "Search the entire web")
   - Complete the setup and get your Search Engine ID (cx)
   - Note: The Search Engine ID looks like: `abcdefghijk`

## Setup

1. Clone the repository:

   ```bash
   git clone https://github.com/idMJA/alya-chatbot.git
   cd rust-chatbot
   ```

2. Create a `.env` file in the project root with your API keys:

   ```rs
   GEMINI_API_KEY=your_gemini_api_key
   GOOGLE_SEARCH_API_KEY=your_google_search_api_key
   GOOGLE_SEARCH_ENGINE_ID=your_search_engine_id
   ```

3. Build the project:

   ```bash
   cargo build --release
   ```

## Usage

Run the chatbot:

```bash
cargo run
```

### First-Time Setup

When you run the chatbot for the first time, it will guide you through setting up your character:

1. Enter a name for your chatbot
2. Describe its personality
3. Provide a description
4. List its traits (comma-separated)
5. List its interests (comma-separated)

### Available Commands

- `learn`: Makes the chatbot search and learn about itself from the web
- `train`: Allows you to train the chatbot with custom text
- `add_url <url>`: Adds a new URL for the chatbot to learn from
- `save`: Saves the current configuration
- `exit`: Quits the chatbot

### Training with Custom Text

When you use the `train` command:

1. Enter your training text
2. Type `END` on a new line when finished
3. The chatbot will process the text and incorporate it into its knowledge

## How It Works

The chatbot uses a combination of techniques to provide intelligent responses:

1. **Character Configuration**: Defines the chatbot's personality and traits
2. **Knowledge Sources**:
   - Web searching via Google Custom Search API
   - URL content extraction and processing
   - Custom text training
3. **AI Processing**: Uses Google's Gemini API to process and personalize information
4. **Conversation Context**: Maintains history to provide context-aware responses
5. **Knowledge Storage**: Persists learned information between sessions

## Project Structure

- `src/main.rs`: Main application code
- `config/chatbot_config.json`: Character and configuration storage
- `data/learned_knowledge.json`: Stored knowledge from learning sessions

## Dependencies

- `reqwest`: HTTP client for API requests
- `tokio`: Async runtime
- `serde`: Serialization/deserialization
- `dotenv`: Environment variable management
- `scraper`: HTML parsing
- `config`: Configuration file handling
- `chrono`: Timestamp generation

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Acknowledgments

- Google Gemini API for natural language processing
- Google Custom Search API for web searching capabilities
