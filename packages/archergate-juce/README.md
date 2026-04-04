# Archergate for JUCE

Drop-in license protection for JUCE audio plugins (VST3, AU, AAX, CLAP).

## Setup

1. Download `archergate_license.lib` / `libarchergate_license.a` and `archergate_license.h` from
   [the latest release](https://github.com/lailaarcher/archergate/releases)

2. Copy `archergate_license.h` and `LicenseManager.h` into your JUCE project's `Source/` directory

3. In Projucer or CMake, add the static library to your linker settings:
   - Windows: `archergate_license.lib`
   - macOS: `libarchergate_license.a` (add to "Extra Frameworks and Libraries")
   - Linux: `libarchergate_license.a` with `-lpthread -ldl -lm`

4. In your `PluginProcessor.cpp` constructor:

```cpp
#include "LicenseManager.h"

MyPluginProcessor::MyPluginProcessor()
{
    licenseManager = std::make_unique<LicenseManager>(
        "your-api-key",
        "com.yourname.plugin"
    );

    // Load saved key from user settings
    juce::PropertiesFile::Options opts;
    opts.applicationName = "MyPlugin";
    opts.folderName = "MyCompany";
    juce::ApplicationProperties props;
    props.setStorageParameters(opts);

    if (auto* settings = props.getUserSettings())
    {
        auto savedKey = settings->getValue("license_key", "");
        if (savedKey.isNotEmpty())
            licenseManager->validate(savedKey);
    }
}
```

5. In `processBlock`, check before processing:

```cpp
void MyPluginProcessor::processBlock(juce::AudioBuffer<float>& buffer, juce::MidiBuffer& midi)
{
    if (!licenseManager->isLicensed() && !licenseManager->isTrialActive())
    {
        buffer.clear(); // Silence output for unlicensed users
        return;
    }

    // Your DSP code here
}
```

6. In your editor, add a button that calls `licenseManager->showLicenseDialog(this)`

## What happens

- First launch: 14-day trial starts automatically
- User buys a key from your store (Gumroad, Stripe, etc.)
- User enters the key in the plugin's license dialog
- Key is validated against your server and locked to their machine
- Key is saved locally. Plugin works offline for 30 days between checks.
- If the user copies the plugin to another machine, the key won't work there.

## No iLok required

No dongle. No iLok account. No PACE integration. A static library that links directly into your plugin binary.
