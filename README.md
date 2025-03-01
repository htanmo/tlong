# tlong

For the times, when your urls are too long.

## Table of Contents

- [tlong](#tlong)
  - [Table of Contents](#table-of-contents)
  - [Installation](#installation)
    - [Prerequisites](#prerequisites)
    - [Steps to install](#steps-to-install)
  - [API Reference](#api-reference)
    - [Base URL](#base-url)
    - [Endpoints](#endpoints)
  - [Examples](#examples)
  - [License](#license)

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
    APP_LOG=trace # (defaults to `log`)
    LOG_DIR=var/log/tlong # (defaults to `./log/`)
    SERVER_ADDRESS=127.0.0.1:3000 # (defaults to `0.0.0.0:8080`)
    REDIS_URL=redis://127.0.0.1:6379
    DATABASE_URL=postgres://username:password@localhost/dbname
    BASE_URL=https://yourdomain.com # (defaults to http://`SERVER_ADDRESS`)
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

## API Reference

### Base URL

`http://localhost:8080/api/v1`

### Endpoints

1. Create Short URL

    `POST /shorten`

    **Request:**
    ```json
    {
        "long_url": "https://example.com/very-long-path"
    }
    ```

    **Response:**
    ```json
    {
        "short_code": "abc12345",
        "short_url": "http://localhost:8080/abc12345",
        "long_url": "https://example.com/very-long-path"
    }
    ```

2. Get All URLs
    
    `GET /shorten`

    **Response:**
    ```json
    [
        {
            "short_code": "abc12345",
            "short_url": "http://localhost:8080/abc12345",
            "long_url": "https://example.com",
            "created_at": "2023-09-20T12:34:56Z"
        }
    ]
    ```

3. Get URL Details
   
    `GET /{short_code}`

    **Response:**
    ```json
    {
        "short_code": "abc12345",
        "short_url": "http://localhost:8080/abc12345",
        "long_url": "https://example.com",
        "created_at": "2023-09-20T12:34:56Z"
    }
    ```

4. Delete URL

    `DELETE /{short_code}`

    **Response:**
    ```json
    {"message": "short url deleted successfully"}
    ```

5. Health Check

    `GET /health`

    **Response:**
    ```json
    {
        "status": "ok",
        "version": "1.0.0"
    }
    ```

## Examples

- **Create Short url**

```sh
curl -X POST http://localhost:8080/api/v1/shorten \
  -H "Content-Type: application/json" \
  -d '{"long_url": "https://example.com"}'
```

- **Redirect Example**

```sh
curl -v http://localhost:8080/abc12345
```

- **Delete URL**

```sh
curl -X DELETE http://localhost:8080/api/v1/abc12345
```

## License

MIT License - see [LICENSE](./LICENSE) for details.
