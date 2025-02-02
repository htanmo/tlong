# tlong

For the times, when your urls are too long.

## Installation

### Prerequisites

- [Rust](https://www.rust-lang.org/)
- [Postgres](https://www.postgresql.org/)
- [Redis](https://redis.io/)

### Steps to install

1. Clone the repository:

    ```sh
    git clone https://github.com/htanmo/tlong.git
    ```

2. Navigate to the project directory:

    ```sh
    cd tlong
    ```

3. Set up the environment variables:

    List of environment variables this project needs.
    ```dotenv
    APP_LOG=trace # Optional (defaults to log)
    SERVER_ADDRESS=127.0.0.1:3000 # Optional (defaults to 0.0.0.0:8080)
    REDIS_URL=redis://127.0.0.1:6379
    DATABASE_URL=postgres://username:password@localhost/dbname
    BASE_URL=https://yourdomain.com (defaults to http://`SERVER_ADDRESS`)
    ```

4. Database setup:

    - Install the `sqlx-cli` tool:
        ```sh
        cargo install sqlx-cli --no-default-features --features postgres
        ```
    
    - Setup the database:
        ```sh
        sqlx database setup
        ```

5. Build the application:

    ```sh
    cargo build --release
    ```

6. Run the server:

    ```sh
    cargo run --release
    ```

    The server will start on `http://0.0.0.0:8080` or `SERVER_ADDRESS` if set.
