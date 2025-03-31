# UPDATE.md - Tencent Meeting Service Change Log

## 2025-03-31: Time Slot Handling Improvements

### Fixed Time Slot Parsing
- Fixed time slot parsing to correctly handle minute precision (e.g., "14:00-14:30")
- Previously only hours were considered, which caused issues with 30-minute slots
- The system now correctly calculates duration based on both hours and minutes

### Added Past Time Adjustment
- Added automatic adjustment for past-time meetings
- If a scheduled time is in the past, it is automatically set to current time + 2 minutes
- Prevents API errors when submitting forms with past times

### Enhanced Time Slot Merging
- Fixed consecutive 30-minute slot identification (e.g., "14:00-14:30" followed by "14:30-15:00")
- These slots are now correctly identified as mergeable and can be combined into single meetings
- Improved debug logging for time range calculations

### Code Cleanup
- Fixed compiler warnings in main.rs
- Improved error handling in the error handler function
- Added comprehensive tests for time slot handling cases