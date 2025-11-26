# Changelog

All notable changes to the Rust USB Bootable Creator project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased - Checkpoint 1.2 Complete]

### Added
- **Complete GUI Modular Architecture**
  - `src/gui/widgets.rs` - 17 widget creation functions for ISO selection, device selection, progress bars, and advanced options
  - `src/gui/events.rs` - 3 event handler functions for button clicks, device refresh, and write operations
  - `src/gui/dialogs.rs` - 5 dialog creation functions for confirmations, errors, and progress feedback
- **Enhanced User Experience**
  - Infinite progress bar with pulsing animation during write operations
  - Real-time status logging for Windows and Linux write processes
  - Enhanced completion dialogs with detailed success/error messages
  - User environment preservation (file picker starts in user home, theme inheritance)
- **Improved Code Organization**
  - Extracted GUI logic from monolithic app.rs into focused modules
  - Separated widget creation from event handling for better maintainability
  - Standardized dialog patterns across the application
- **Linux persistence robustness**
  - Requires `sgdisk`/gptfdisk for GPT repair after ISO write
  - Refresh loop with `partprobe`/`udevadm settle` and free-space validation before creating the persistence partition
- **Partition table choice**
  - Advanced option to select GPT (default) or MBR for persistence; backend validates against the current device table
- **Development guidelines in CLAUDE.md**
- **Agent handoff protocols in AGENTS.md**
- **Comprehensive development plan in PLAN.md**

### Fixed
- GTK4 threading compatibility issues with progress updates
- Lifetime and borrowing issues in event handlers
- Type conversion issues for cluster size configuration
- Dialog builder compatibility with GTK4
- All compilation errors - application builds successfully with only warnings

### Current State
- ✅ Windows ISO creation fully functional with proper dual-partition layout
- ✅ Linux ISO support working with basic dd-based writing
- ✅ Complete GUI implementation with modular architecture
- ⚠️ GitHub Actions working for DEB and Flatpak, RPM broken
- ❌ Linux persistence checkbox exists but backend not implemented

## [Previous Versions - Based on Git History]

### [f1fc226] - Refactor ISO writing functions
- Removed unnecessary privilege escalation
- Streamlined commands for Linux and Windows flows
- Simplified command execution patterns

### [36bda57] - Enhance ISO detection and GUI options
- Improved Windows and Linux ISO detection
- Enhanced GUI options for both ISO types
- Refactored mounting logic and improved user experience

### [3bbdd3b] - Remove unused IDE configuration files
- Cleaned up project structure
- Removed obsolete configuration files

### [6eb36b4] - Remove unused files
- Further cleanup of project structure

### [fac9a83] - Revamp GUI with device/ISO selection
- Implemented device and ISO selection in GUI
- Removed unused modules
- Improved user interface design

## Planned Changes (Based on PLAN.md)

### Phase 1 - Critical Fixes and Refactoring
- Code architecture refactoring
- GUI module completion
- Linux persistence implementation
- GitHub Actions fixes

### Phase 2 - Windows ISO Customization
- TPM 2.0 bypass implementation
- Secure Boot requirement removal
- RAM requirement reduction
- OOBE automation features

### Phase 3 - Enhanced Features
- Advanced format options (exFAT, ext2/3/4)
- Bad blocks checking
- Enhanced ISO detection and validation
- Performance improvements

### Phase 4 - Multi-platform Support
- Windows compatibility (WSL)
- macOS support

### Phase 5 - Quality Assurance
- Comprehensive testing suite
- Security audit
- Documentation completion

## Technical Debt

### Immediate Attention Required
- [x] Empty GUI modules (widgets.rs, events.rs, dialogs.rs) - COMPLETED
- [ ] Code duplication in flow modules (moved to backlog)
- [ ] Missing Linux persistence implementation
- [ ] Broken RPM package creation in GitHub Actions

### Code Quality Improvements Needed
- [ ] Extract hardcoded constants to config.rs
- [ ] Implement custom error types
- [ ] Add comprehensive unit and integration tests
- [ ] Improve error messaging with context

### Documentation Gaps
- [ ] API documentation for public functions
- [ ] User guide for advanced features
- [ ] Troubleshooting documentation
- [ ] Development setup instructions

## Version Information

**Current Development Branch**: fix/gui_modular
**Target Next Version**: To be determined based on Phase 1 completion
**Stability**: Basic functionality stable, advanced features in development

## Breaking Changes (Anticipated)

Future versions may include breaking changes for:
- Configuration file format
- CLI argument structure
- GUI module interfaces
- Package dependencies

All breaking changes will be documented in detail in their respective release notes.
