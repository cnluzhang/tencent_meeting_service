# UPDATE.md - Tencent Meeting Service Change Log

## 2025-04-01: Time Slot Validation Improvements

### Added Error Handling for Completely Past Time Slots
- Changed behavior to return an error when both start and end times are in the past
- Previously would adjust with a 5-minute minimum duration, now properly rejects invalid time slots
- This prevents creation of very short meetings when both times are in the past
- Updated tests to verify the new error condition
- The form service already prevents this scenario, this change adds an additional layer of validation

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

### Fixed Past Time Slot Issues
- Fixed a critical bug that prevented consecutive time slots from merging when some were in the past
- Modified time slot adjustment to preserve original end times for past time slots
- Only adjusts start times of past slots to current time + 2 minutes, keeping end times intact 
- Ensures consecutive time slots remain mergeable even when the first slot was in the past
- Added comprehensive tests for past time handling and slot merging scenarios

### Code Cleanup
- Fixed compiler warnings in main.rs
- Improved error handling in the error handler function
- Added comprehensive tests for time slot handling cases