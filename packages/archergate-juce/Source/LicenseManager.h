#pragma once

// Archergate License Manager for JUCE plugins
// Drop this file into your JUCE project Source directory.
// Call LicenseManager::validate() in your PluginProcessor constructor.
//
// Link against: archergate_license.lib (Windows) or libarchergate_license.a (Unix)
// Download from: https://github.com/lailaarcher/archergate/releases

#include "archergate_license.h"
#include <JuceHeader.h>

class LicenseManager
{
public:
    LicenseManager(const juce::String& apiKey, const juce::String& pluginId)
    {
        client = ag_license_new(apiKey.toRawUTF8(), pluginId.toRawUTF8());
    }

    ~LicenseManager()
    {
        if (client != nullptr)
            ag_license_free(client);
    }

    // Call this in your PluginProcessor constructor.
    // Returns true if licensed, false if not.
    bool validate(const juce::String& licenseKey)
    {
        if (client == nullptr)
            return false;

        int result = ag_license_validate(client, licenseKey.toRawUTF8());
        licensed = (result == AG_LICENSE_OK);
        return licensed;
    }

    // Call this to check trial status.
    bool isTrialActive()
    {
        if (client == nullptr)
            return false;

        int days = ag_license_trial_days_remaining(client);
        return days > 0;
    }

    int trialDaysRemaining()
    {
        if (client == nullptr)
            return 0;

        return ag_license_trial_days_remaining(client);
    }

    bool isLicensed() const { return licensed; }

    // Show a license dialog. Wire this to a button in your editor.
    void showLicenseDialog(juce::Component* parent)
    {
        auto* dialog = new juce::AlertWindow(
            "Enter License Key",
            "Paste your license key below.\nPurchase at: https://your-store.com",
            juce::MessageBoxIconType::QuestionIcon,
            parent
        );

        dialog->addTextEditor("key", "", "License Key");
        dialog->addButton("Activate", 1);
        dialog->addButton("Cancel", 0);

        dialog->enterModalState(true, juce::ModalCallbackFunction::create(
            [this, dialog](int result)
            {
                if (result == 1)
                {
                    auto key = dialog->getTextEditorContents("key");
                    if (validate(key))
                    {
                        // Save key to user settings
                        juce::PropertiesFile::Options opts;
                        opts.applicationName = "YourPlugin";
                        opts.folderName = "YourCompany";
                        juce::ApplicationProperties props;
                        props.setStorageParameters(opts);

                        if (auto* settings = props.getUserSettings())
                        {
                            settings->setValue("license_key", key);
                            settings->saveIfNeeded();
                        }

                        juce::AlertWindow::showMessageBoxAsync(
                            juce::MessageBoxIconType::InfoIcon,
                            "Activated",
                            "License activated. Thank you."
                        );
                    }
                    else
                    {
                        juce::AlertWindow::showMessageBoxAsync(
                            juce::MessageBoxIconType::WarningIcon,
                            "Invalid Key",
                            "That key didn't work. Check for typos or contact support."
                        );
                    }
                }
                delete dialog;
            }
        ));
    }

private:
    AgLicenseClient* client = nullptr;
    bool licensed = false;

    JUCE_DECLARE_NON_COPYABLE_WITH_LEAK_DETECTOR(LicenseManager)
};
