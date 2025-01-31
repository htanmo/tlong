# tlong

For the times, when your urls are too long.

## Installation

### Prerequisites

- [Rust](https://www.rust-lang.org/)
- [Postgres](https://www.postgresql.org/)

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
    DATABASE_URL=postgres://username:password@localhost/dbname
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

