# Almambet - Email Reader and Web Interface

A Rust application that provides both command-line and web interface for reading emails from an IMAP server.

## About the Name

The name "Almambet" is inspired by a significant character from the Kyrgyz epic poem "Manas". Almambet was a loyal friend and trusted advisor to Manas, the epic's protagonist. He was known for his wisdom, bravery, and unwavering loyalty. Like the character Almambet who served as a reliable messenger and advisor, this application serves as a trustworthy tool for managing and delivering your email communications.

## Features

### Email Reading
- Connects to IMAP servers securely using TLS
- Fetches recent emails from the INBOX
- Extracts email metadata including:
  - Subject
  - From
  - Date
  - To
  - CC
  - BCC
  - Reply-To
  - Message ID
  - Content Type
  - Message Content
- Secure password storage using AES-256-GCM encryption
- Automatic password caching (only asks once)

### Web Interface
- Modern, responsive web interface
- JSON-based message display
- Real-time email fetching
- Clean and intuitive UI

## Configuration

The application requires a `config.yaml` file with the following structure:

```yaml
email_address: "your.email@example.com"
imap_server: "imap.example.com"
port: 993
```

## Security Features

- TLS encryption for IMAP connections
- AES-256-GCM encryption for stored passwords
- Secure key management
- Password caching with encryption

## Usage

### Command Line Interface

```bash
cargo run
```

This will:
1. Connect to your configured IMAP server
2. Fetch the 10 most recent emails
3. Display them in JSON format

### Web Interface

```bash
cargo run --bin web
```

This will:
1. Start a web server on port 3000
2. Fetch emails from your IMAP server
3. Display them in a web interface at http://localhost:3000

## Dependencies

- `async-imap`: For IMAP server communication
- `tokio`: For async runtime
- `mailparse`: For email parsing
- `aes-gcm`: For password encryption
- `axum`: For web server
- `tera`: For HTML templating
- `serde`: For serialization/deserialization
- `anyhow`: For error handling

## Project Structure

```
src/
├── main.rs           # Main entry point
├── mail_reader.rs    # Email fetching and processing
├── web.rs           # Web server implementation
├── settings.rs      # Configuration handling
└── templates/       # HTML templates
    └── emails.html  # Email display template
```

## Security Notes

- The application stores encrypted passwords in `.encrypted_password`
- Encryption keys are stored in `.encryption_key`
- Both files should be kept secure and not shared
- The password is only requested once and then cached securely
