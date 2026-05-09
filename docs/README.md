# Arc Documentation

Welcome to the Arc documentation. This documentation provides comprehensive information about the architecture, implementation, and usage of the Arc web application framework.

## Table of Contents

1. **[Overview](01-overview.md)**
   - Introduction and project information
   - Key features and technology stack
   - Quick start guide
   - Directory structure

2. **[Architecture](02-architecture.md)**
   - MVC architecture overview
   - Design patterns used
   - Request flow
   - Module organization
   - Configuration

3. **[Backend](03-backend.md)**
   - Entry point and CLI commands
   - Routing configuration
   - Controllers (Home, Auth, Admin)
   - Middleware (AuthMiddleware)
   - Services and Helpers
   - Models
   - Development mode

4. **[Frontend](04-frontend.md)**
   - Template engine (Tera)
   - Styling with Tailwind CSS
   - JavaScript (Alpine.js, HTMX)
   - Build system (Vite)
   - Asset management
   - Admin panel layout

5. **[Database](05-database.md)**
   - SQLite configuration
   - Diesel ORM
   - Migrations
   - Models and CRUD operations
   - Seeders
   - Testing database

6. **[Testing](06-testing.md)**
   - Test configuration
   - Test utilities
   - Test patterns
   - Examples for models, controllers, middleware
   - Best practices

7. **[API Reference](07-api-reference.md)**
   - Public endpoints
   - Protected endpoints
   - Request/response formats
   - Data types
   - Error handling

8. **[Problems and Improvements](08-problems-and-improvements.md)**
   - Critical issues
   - Security concerns
   - Code quality issues
   - Low-hanging fruit improvements
   - Performance optimizations

## Quick Links

### Getting Started

- [Quick Start Guide](01-overview.md#quick-start)
- [System Requirements](01-overview.md#system-requirements)

### Development

- [CLI Commands](03-backend.md#cli-commands)
- [Development Mode](03-backend.md#development-mode)
- [Creating New Pages](04-frontend.md#adding-new-pages)
- [Database Migrations](05-database.md#migrations)

### Reference

- [API Endpoints](07-api-reference.md)
- [Database Schema](05-database.md#tables)

### Maintenance

- [Known Issues](08-problems-and-improvements.md#critical-issues)
- [Security Recommendations](08-problems-and-improvements.md#security-concerns)

## Organizing this Documentation

This documentation is set to use Docsify, which allows for easy navigation and organization. Each section is designed to be self-contained, providing detailed information on specific aspects of the project. The structure is intended to guide both new contributors and experienced developers through the codebase effectively. It currently use for diagrams this solution that must be present in the current docs directory: https://github.com/firebrikdotcom/diagram-management.

## Contributing

When contributing to this project, please:

1. Read the [Architecture](02-architecture.md) document to understand the codebase structure
2. Review [Problems and Improvements](08-problems-and-improvements.md) for known issues
3. Follow existing code patterns documented in [Backend](03-backend.md) and [Frontend](04-frontend.md)
4. Add tests as documented in [Testing](06-testing.md)

## License

This project is licensed under the MIT License.
