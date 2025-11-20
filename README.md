# Almambet - Email Reader

Email reader application built with Rust, featuring IMAP support, a web interface and a REST one.

## About the Name

The name "Almambet" is inspired by a significant character from the Kyrgyz epic poem "Manas". Almambet was a loyal friend and trusted advisor to Manas, the epic's protagonist. He was known for his wisdom, bravery, and unwavering loyalty. Like the character Almambet who served as a reliable messenger and advisor, this application serves as a trustworthy tool for managing and delivering your email communications.

## Configuration

The application will generate the following encrypted files and use them as a cache of your password:

- `.encrypted_password`: Encrypted email password
- `.encryption_key`: Encryption key for credentials

## Building and Running

1. Clone the repository:
   ```bash
   git clone https://github.com/yourusername/almambet.git
   cd almambet
   ```

2. Build the project:
   ```bash
   cargo build --release
   ```

3. Modify the configuration files inside the `resources` folder.

3. Run the program.

    * Run the spam filter once and exit:
      ```bash
      cargo run -- --once
      ```

    * Delete spam messages and exit:
      ```bash
      cargo run -- --spam
      ```

    * Run the web interface and spam filter at the interval specified in the configuration files:
      ```bash
      cargo run -- --periodic --web
      ```    

    * Run the REST interface and spam filter at the interval specified in the configuration files:
      ```bash
      cargo run -- --periodic --rest
      ```

The web interface will be available at `http://localhost:3000`.

The REST interface will be available, for example, at `http://localhost:3000/api/v1/emails/INBOX` for the INBOX folder.


## Configuration

The application requires a `settings.yaml` file with the following structure:

```yaml
imap:
  server: "imap.example.com"
  port: 993
  username: "user@example.com"

# Massage filter settings
mail_mover:
  check_interval: 60                # Check interval in seconds

# REST API server configuration
server:
  host: "0.0.0.0"                  # Bind to all network interfaces
  port: 3000
```
It is also required an `email_move_rules.yaml` file like this:

```yaml
messages_to_check: 1500
rules:
  - rule:
      target_folder: "Spam"
      from:
        - "@phishing\\.net>$"
        - "@scam\\.xyz>$"
        - "promo@shopping\\.biz>$"
        - "noreply@lottery\\.win>$"
        - "account@fakebank\\.com>$"
      
      title:
        - "URGENT: Account Verification Required"
        - "You Won a Prize!"
        - "Limited Time Offer"
        - "Exclusive Deal Just For You"
        - ".*password.*expir.*"
      
      body:
        - "click here to claim your prize"
        - "limited time offer, act now"
        - "your account will be suspended"
        - "verify your identity immediately"
        - "special discount just for you"
```