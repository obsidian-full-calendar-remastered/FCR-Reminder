# 🎉 Contributing to Full Calendar

Full Calendar is open to contributions, and we’re excited to have you here! This guide will help you get set up for local development.

## Important Information for Contributors:

- **Respect code architecture:** Please follow the established architecture and modular design of the project. Avoid placing code arbitrarily; keep changes organized and maintainable.

- **Dont Modify test files:** Do **not** modify any existing `.test` files. You may add new tests, but existing tests should remain unchanged. This allows core maintainers to easily identify breaking changes and ensures they are responsible for updating tests if needed.

- **Build configuration:** Avoid changing build configuration files unless absolutely necessary. Sudden build changes can disrupt other contributors, as many have their own development setups.

- **Document your changes:** Update documentation (README, docs, or code comments) when you add features or change existing behavior, so that other collaborators can easily catch up.

- **Test your changes:** Add or update tests for your code. Make sure all tests pass before submitting a PR. If some tests do not pass and you have good reason to believe the tests themselves are incorrect, mention this in your PR description, but do not change the tests (core maintainers will handle test updates). Run both `npm run test` and `npm run lint:eslint` before submitting a PR.

---

## 🚀 Getting Started

### 1. Create the Obsidian Vault

To develop locally, set up your development vault and plugin directory:

```bash
mkdir -p .obsidian/.plugins/full-calendar-remastered/
cp manifest.json .obsidian/.plugins/full-calendar-remastered/manifest.json
````

*Currently this folder already exists and will contain the minimified builds accordingly the latest tags (this is done to simplify the obsidian community plugin release process).

> 💡 **Note:** Obsidian expects a CSS file named `styles.css`, but **esbuild** will output one named `main.css`.

---

### 2. Build the Plugin

You can build the plugin in two ways:

* For development:

  ```bash
  npm run dev
  ```

* For a production/minified build:

  ```bash
  npm run prod
  ```

All build output will appear in the plugin directory created above.

---

### 3. Open the Vault in Obsidian

1. Open **Obsidian**
2. Go to **Vaults** → **Open Folder as Vault**
3. Select the `obsidian-dev-vault` directory

---

## 🧠 Tips for Developers

> 💡 **Recommended:** Use the [Hot Reload plugin](https://github.com/pjeby/hot-reload) to make development smoother — it auto-reloads your plugin changes.

> 📘 **Start Here:** To understand the architecture and get familiar with the codebase, read our [Architecture Guide](https://github.com/obsidian-full-calendar-remastered/plugin-full-calendar/blob/main/src/README.md).

> 📱 **Android Testing** For testing Android devices use `adb` together with `chrome://inspect/#devices` to see the console on the PC.

---

Thanks for helping improve Full Calendar! 🎨🗓️
