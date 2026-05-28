.PHONY: help install build dev serve migrate seed test test-e2e test-confidence clean format check lint docker-build docker-up docker-down

# Default target
.DEFAULT_GOAL := help

# Colors for output
CYAN := \033[0;36m
GREEN := \033[0;32m
YELLOW := \033[0;33m
NC := \033[0m # No Color

##@ General

help: ## Display this help message
	@awk 'BEGIN {FS = ":.*##"; printf "\nUsage:\n  make $(CYAN)<target>$(NC)\n"} /^[a-zA-Z_-]+:.*?##/ { printf "  $(CYAN)%-15s$(NC) %s\n", $$1, $$2 } /^##@/ { printf "\n$(YELLOW)%s$(NC)\n", substr($$0, 5) } ' $(MAKEFILE_LIST)

##@ Setup & Installation

# Install all project dependencies including Rust crates, npm packages, and cargo-watch
install: ## Install all dependencies (Rust, npm, and cargo-watch)
	@echo "$(GREEN)Installing Rust dependencies...$(NC)"
	cargo fetch
	@echo "$(GREEN)Installing npm dependencies...$(NC)"
	npm install
	@echo "$(GREEN)Checking for cargo-watch...$(NC)"
	@if ! command -v cargo-watch >/dev/null 2>&1; then \
		echo "$(YELLOW)Installing cargo-watch...$(NC)"; \
		cargo install cargo-watch; \
	else \
		echo "$(GREEN)cargo-watch already installed$(NC)"; \
	fi
	@echo "$(GREEN)Dependencies installed successfully!$(NC)"

# Verify required system packages: build-essential, libssl-dev, libsqlite3-dev
deps-check: ## Check if all required system dependencies are installed
	@echo "$(GREEN)Checking system dependencies...$(NC)"
	@command -v gcc >/dev/null 2>&1 || (echo "$(YELLOW)Warning: build-essential not found$(NC)" && exit 1)
	@pkg-config --exists openssl 2>/dev/null || (echo "$(YELLOW)Warning: libssl-dev not found$(NC)" && exit 1)
	@pkg-config --exists sqlite3 2>/dev/null || (echo "$(YELLOW)Warning: libsqlite3-dev not found$(NC)" && exit 1)
	@echo "$(GREEN)All system dependencies are installed$(NC)"

##@ Development

# Run dev server with hot-reload, watches Rust code changes and bundles frontend with Vite
dev: ## Start development server with hot-reload and Vite bundling
	@echo "$(GREEN)Starting development server...$(NC)"
	cargo run develop

# Start production server without file watching or hot-reload
serve: ## Start production server without hot-reload
	@echo "$(GREEN)Starting production server...$(NC)"
	cargo run serve

# Compile optimized binary with full optimizations (slower build, faster runtime)
build: ## Build the project in release mode
	@echo "$(GREEN)Building project in release mode...$(NC)"
	cargo build --release

# Compile unoptimized binary for faster compilation during development
build-dev: ## Build the project in debug mode
	@echo "$(GREEN)Building project in debug mode...$(NC)"
	cargo build

# Continuously check code and run tests on file changes using cargo-watch
watch: ## Watch for changes and rebuild (requires cargo-watch)
	@echo "$(GREEN)Watching for changes...$(NC)"
	cargo watch -x check -x test

##@ Database

# Apply all pending Diesel migrations to update database schema
migrate: ## Run database migrations
	@echo "$(GREEN)Running database migrations...$(NC)"
	cargo run migrate

# Populate database with initial sample data for development
seed: ## Seed the database with sample data
	@echo "$(GREEN)Seeding database...$(NC)"
	cargo run seed

# Run migrations then seed - complete database initialization
db-setup: migrate seed ## Setup database (migrate + seed)
	@echo "$(GREEN)Database setup complete!$(NC)"

# Generate new Diesel migration file. Usage: make diesel-migration NAME=create_users
diesel-migration: ## Create a new Diesel migration (usage: make diesel-migration NAME=migration_name)
	@if [ -z "$(NAME)" ]; then \
		echo "$(YELLOW)Error: Please provide NAME parameter$(NC)"; \
		echo "Usage: make diesel-migration NAME=create_users"; \
		exit 1; \
	fi
	diesel migration generate $(NAME)

##@ Testing & Quality

# Execute all unit and integration tests
test: ## Run all tests
	@echo "$(GREEN)Running tests...$(NC)"
	cargo test

# Run browser-level confidence tests against a real app server using Playwright
test-e2e: ## Run Playwright end-to-end confidence tests
	@echo "$(GREEN)Running Playwright confidence tests...$(NC)"
	npm run test:e2e

# Run both Rust tests and browser confidence tests
test-confidence: ## Run Rust tests and Playwright confidence tests
	@echo "$(GREEN)Running full confidence suite...$(NC)"
	cargo test
	npm run test:e2e

# Run tests with full output including println! statements
test-verbose: ## Run tests with verbose output
	@echo "$(GREEN)Running tests (verbose)...$(NC)"
	cargo test -- --nocapture

# Generate HTML coverage report using cargo-tarpaulin (auto-installs if needed)
coverage: ## Generate test coverage report (requires cargo-tarpaulin)
	@echo "$(GREEN)Generating coverage report...$(NC)"
	@if ! command -v cargo-tarpaulin >/dev/null 2>&1; then \
		echo "$(YELLOW)Installing cargo-tarpaulin...$(NC)"; \
		cargo install cargo-tarpaulin; \
	fi
	cargo tarpaulin --out Html --output-dir coverage

# Verify code compiles without producing binary (fast validation)
check: ## Check code without building
	@echo "$(GREEN)Checking code...$(NC)"
	cargo check

# Auto-format all Rust code according to rustfmt style guidelines
format: ## Format code using rustfmt
	@echo "$(GREEN)Formatting code...$(NC)"
	cargo fmt

# Check if code is formatted correctly without modifying files
format-check: ## Check code formatting without making changes
	@echo "$(GREEN)Checking code format...$(NC)"
	cargo fmt -- --check

# Run Clippy linter with warnings treated as errors for strict quality checks
lint: ## Run clippy linter
	@echo "$(GREEN)Running clippy...$(NC)"
	cargo clippy -- -D warnings

# Scan dependencies for known security vulnerabilities (auto-installs cargo-audit)
audit: ## Audit dependencies for security vulnerabilities
	@echo "$(GREEN)Auditing dependencies...$(NC)"
	@if ! command -v cargo-audit >/dev/null 2>&1; then \
		echo "$(YELLOW)Installing cargo-audit...$(NC)"; \
		cargo install cargo-audit; \
	fi
	cargo audit

##@ Frontend

# Bundle and minify CSS/JS assets for production using Vite
frontend-build: ## Build frontend assets with Vite
	@echo "$(GREEN)Building frontend assets...$(NC)"
	npm run build

# Start Vite dev server with hot module replacement for frontend development
frontend-dev: ## Run Vite development server
	@echo "$(GREEN)Starting Vite development server...$(NC)"
	npm run dev

# Install only npm/JavaScript dependencies (Tailwind, Vite, Hotwire)
frontend-install: ## Install npm dependencies only
	@echo "$(GREEN)Installing npm dependencies...$(NC)"
	npm install

##@ Cleanup

# Remove all compiled artifacts, node_modules, and build outputs
clean: ## Remove build artifacts and dependencies
	@echo "$(GREEN)Cleaning build artifacts...$(NC)"
	cargo clean
	rm -rf node_modules
	rm -rf dist
	rm -rf target

# Delete SQLite database files with confirmation prompt (DESTRUCTIVE)
clean-db: ## Remove database file (WARNING: destructive)
	@echo "$(YELLOW)Warning: This will delete your database!$(NC)"
	@read -p "Are you sure? [y/N] " -n 1 -r; \
	echo; \
	if [[ $$REPLY =~ ^[Yy]$$ ]]; then \
		rm -f *.db *.db-shm *.db-wal; \
		echo "$(GREEN)Database files removed$(NC)"; \
	fi

##@ Utilities

# Execute arbitrary cargo commands. Usage: make run CMD="serve" or make run CMD="custom-command"
run: ## Run custom cargo command (usage: make run CMD="your command")
	@if [ -z "$(CMD)" ]; then \
		echo "$(YELLOW)Usage: make run CMD=\"your command\"$(NC)"; \
		exit 1; \
	fi
	cargo run $(CMD)

# Tail application log files if logs directory exists
logs: ## View application logs (if implemented)
	@echo "$(GREEN)Viewing logs...$(NC)"
	@if [ -d "logs" ]; then \
		tail -f logs/*.log; \
	else \
		echo "$(YELLOW)No logs directory found$(NC)"; \
	fi

# One-command setup for new developers: checks deps, installs packages, initializes database
setup: deps-check install db-setup ## Complete project setup for new developers
	@echo "$(GREEN)============================================$(NC)"
	@echo "$(GREEN)Setup complete! You can now run:$(NC)"
	@echo "  $(CYAN)make dev$(NC)     - Start development server"
	@echo "  $(CYAN)make test$(NC)    - Run tests"
	@echo "$(GREEN)============================================$(NC)"

##@ Environment

# Create .env from .env.example template if not exists
env-copy: ## Copy .env.example to .env
	@if [ ! -f .env ]; then \
		echo "$(GREEN)Creating .env file from .env.example...$(NC)"; \
		cp .env.example .env; \
		echo "$(YELLOW)Remember to update .env with your settings!$(NC)"; \
	else \
		echo "$(YELLOW).env file already exists$(NC)"; \
	fi

# Copy test environment configuration for running tests
env-test: ## Set up test environment
	@echo "$(GREEN)Setting up test environment...$(NC)"
	@if [ -f .env.test ]; then \
		cp .env.test .env; \
		echo "$(GREEN)Test environment configured$(NC)"; \
	else \
		echo "$(YELLOW).env.test not found$(NC)"; \
	fi
