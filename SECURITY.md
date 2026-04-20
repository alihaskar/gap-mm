# Security Policy

## Reporting a vulnerability

Please **do not** file a public GitHub issue for security vulnerabilities.

Email: 26202651+alihaskar@users.noreply.github.com

Include:
- A description of the vulnerability
- Steps to reproduce
- Potential impact

You will receive a response within 72 hours.

## API key handling

- API keys are read from environment variables / `.env` file at startup.
- Keys are never logged, printed, or stored beyond process memory.
- The `.env` file is in `.gitignore` — never commit it.
- Use a **dedicated Bybit sub-account** with trading permissions only (no withdrawal permission).
- Set IP whitelisting on your API key if your server has a static IP.

## Dependency security

- Rust dependencies are pinned in `Cargo.lock` (gitignored by default — consider committing it for reproducibility).
- Python dependencies are managed by Poetry (`pyproject.toml`).
- The `OrderBook-rs` submodule is third-party code (MIT). Review its releases before updating.

## Financial risk

This software places real orders on a live exchange. A bug, misconfiguration, or network issue can cause financial loss. Always:

- Test with the smallest possible `MIN_ORDER_SIZE` and `MAX_POSITION_SIZE`.
- Monitor the bot manually for the first session.
- Set conservative position limits.
- Have a kill switch ready (Ctrl+C, or manually cancel all orders on the exchange).
