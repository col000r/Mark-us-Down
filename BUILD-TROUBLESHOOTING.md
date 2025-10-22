# Build Troubleshooting Guide

## Critical Issue: Silent Build Failures

### The Problem

When running `CI=true npm run tauri build`, TypeScript compilation errors are **silently ignored** and the build uses **old cached files** from the `dist/` directory. This means:

1. You make code changes
2. Build appears to succeed
3. But the .app/.dmg contains OLD code
4. Features don't work and there's no error message

### Root Cause

The `tauri build` command with `CI=true` continues even if the frontend build (`npm run build`) fails. If there are TypeScript errors:
- The `tsc` step fails
- Vite doesn't run
- Old files in `dist/` are bundled instead
- No error is shown in the final output

### How to Prevent This

**Always run a clean build when testing changes:**

```bash
npm run clean && CI=true npm run tauri build
```

Or separately:

```bash
# 1. Clean first
npm run clean

# 2. Test frontend build separately to catch errors
npm run build

# 3. If that succeeds, build the full app
CI=true npm run tauri build
```

### Warning Signs

- Build completes very quickly (< 5 seconds for frontend)
- File timestamps in `dist/assets/` are old
- Changes don't appear in the built app
- No TypeScript errors shown despite syntax issues

### The Fix

The `clean` script in `package.json` removes both build directories:

```json
"clean": "rm -rf dist && rm -rf src-tauri/target"
```

This ensures:
- No stale frontend code in `dist/`
- No stale Rust binaries in `src-tauri/target/`
- Fresh compilation of everything

### Best Practice Workflow

1. **During development**: Use `npm run tauri:dev` which auto-reloads
2. **Before testing a build**: Always run `npm run clean` first
3. **Check for TypeScript errors**: Run `npm run typecheck` or `npm run build` alone
4. **Verify build freshness**: Check timestamps with `ls -lh dist/assets/*.js`

### Example: The Scroll Sync Bug

During the scroll sync debugging, we had:
- Missing `useEffect` import in `SourceEditor.tsx`
- TypeScript error: `Cannot find name 'useEffect'`
- Build continued with old code from `dist/`
- Debug features never appeared in the app
- Took many iterations to discover the issue

**Solution**: Running `npm run clean && npm run build` revealed the TypeScript error immediately.

## Summary

**Always clean before building when testing changes:**
```bash
npm run clean && CI=true npm run tauri build
```

This prevents hours of debugging phantom issues caused by stale cached files.
