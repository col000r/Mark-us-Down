# How to Update the Version Number

When releasing a new version of Mark-us-Down, you need to update the version number in **four** locations:

## Files to Update

1. **`src-tauri/tauri.conf.json`** (line 4)
   ```json
   "version": "X.Y.Z",
   ```

2. **`src-tauri/Cargo.toml`** (line 3)
   ```toml
   version = "X.Y.Z"
   ```

3. **`package.json`** (line 4)
   ```json
   "version": "X.Y.Z",
   ```

4. **`src/App.tsx`** (around line 791, in the About dialog)
   ```tsx
   <p className="version">Version X.Y.Z</p>
   ```

## Quick Update Process

1. **Search for the current version** across all files:
   ```bash
   grep -r "1.0.3" --include="*.json" --include="*.toml" --include="*.tsx"
   ```

2. **Update each file** with the new version number

3. **Verify all occurrences are updated**:
   ```bash
   # Should return no results for the old version
   grep -r "1.0.2" --include="*.json" --include="*.toml" --include="*.tsx"
   ```

4. **Build the application**:
   ```bash
   CI=true npm run tauri build
   ```

## Notes

- The version in `Cargo.toml` and `tauri.conf.json` determines the filename of the generated DMG (e.g., `Mark-us-Down_1.0.3_aarch64.dmg`)
- The version in `App.tsx` is what users see in the About dialog
- Always use semantic versioning: `MAJOR.MINOR.PATCH`
- Remember to set `CI=true` when building to avoid DMG bundling issues on macOS
