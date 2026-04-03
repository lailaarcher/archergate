// Serverless API endpoint for SDK beta signup
// Deploy as: Vercel serverless function, Netlify function, or Cloudflare Worker
// Sends SDK download email via Resend (https://resend.com)
//
// Environment variables required:
//   RESEND_API_KEY — your Resend API key (re_xxxxxxxxxx)
//
// Setup:
//   1. Sign up at resend.com
//   2. Verify your domain (archergate.io)
//   3. Create an API key
//   4. Set RESEND_API_KEY in your deployment environment

export default async function handler(req, res) {
    if (req.method !== 'POST') {
        return res.status(405).json({ error: 'Method not allowed' });
    }

    const { name, email, app_name, app_id, software_type, source } = req.body;

    if (!name || !email || !app_name) {
        return res.status(400).json({ error: 'Missing required fields' });
    }

    const RESEND_API_KEY = process.env.RESEND_API_KEY;
    if (!RESEND_API_KEY) {
        console.error('RESEND_API_KEY not set');
        return res.status(500).json({ error: 'Server configuration error' });
    }

    // Send email via Resend
    try {
        const emailResp = await fetch('https://api.resend.com/emails', {
            method: 'POST',
            headers: {
                'Authorization': `Bearer ${RESEND_API_KEY}`,
                'Content-Type': 'application/json',
            },
            body: JSON.stringify({
                from: 'Archergate <hello@archergate.io>',
                to: [email],
                subject: 'Your Archergate License SDK',
                html: buildEmailHTML(name, app_name, app_id, software_type),
            }),
        });

        if (!emailResp.ok) {
            const errBody = await emailResp.text();
            console.error('Resend error:', errBody);
            return res.status(500).json({ error: 'Failed to send email' });
        }

        // Log signup (optional: store in database later)
        console.log(`SDK beta signup: ${email} | ${app_name} | ${software_type} | ${source}`);

        return res.status(200).json({ ok: true });
    } catch (err) {
        console.error('Email send error:', err);
        return res.status(500).json({ error: 'Internal server error' });
    }
}

function buildEmailHTML(name, appName, appId, softwareType) {
    const firstName = name.split(' ')[0];

    return `
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
</head>
<body style="margin:0;padding:0;background:#F5F3EF;font-family:Georgia,'Times New Roman',serif;">
    <table width="100%" cellpadding="0" cellspacing="0" style="background:#F5F3EF;padding:40px 20px;">
        <tr><td align="center">
            <table width="600" cellpadding="0" cellspacing="0" style="background:#F5F3EF;">

                <!-- Header -->
                <tr><td style="padding:0 0 20px;border-bottom:1px solid #D4D0C8;">
                    <span style="font-family:'Courier New',monospace;font-size:14px;letter-spacing:0.15em;color:#222;">A R C H E R G A T E</span>
                </td></tr>

                <!-- Body -->
                <tr><td style="padding:40px 0;">
                    <h1 style="font-family:Georgia,serif;font-size:32px;font-style:italic;font-weight:normal;color:#222;margin:0 0 24px;">You're in.</h1>

                    <p style="font-family:Georgia,serif;font-size:16px;color:#444;line-height:1.7;margin:0 0 20px;">
                        Here are your download links for ${appName}. Pick your platform, extract, and link the static library into your project. The header file is included.
                    </p>

                    <table width="100%" cellpadding="0" cellspacing="0" style="margin:24px 0;">
                        <tr><td style="padding:12px 0;border-bottom:1px solid #E8E5DF;">
                            <span style="font-family:'Courier New',monospace;font-size:12px;color:#888;">WINDOWS (MSVC)</span><br>
                            <a href="https://github.com/lailaarcher/archergate/releases/latest/download/archergate-license-windows-x64.tar.gz" style="font-family:Georgia,serif;font-size:15px;color:#222;text-decoration:underline;">Download .tar.gz</a>
                        </td></tr>
                        <tr><td style="padding:12px 0;border-bottom:1px solid #E8E5DF;">
                            <span style="font-family:'Courier New',monospace;font-size:12px;color:#888;">MACOS (INTEL + APPLE SILICON)</span><br>
                            <a href="https://github.com/lailaarcher/archergate/releases/latest/download/archergate-license-macos-universal.tar.gz" style="font-family:Georgia,serif;font-size:15px;color:#222;text-decoration:underline;">Download .tar.gz</a>
                        </td></tr>
                        <tr><td style="padding:12px 0;border-bottom:1px solid #E8E5DF;">
                            <span style="font-family:'Courier New',monospace;font-size:12px;color:#888;">LINUX (X86_64)</span><br>
                            <a href="https://github.com/lailaarcher/archergate/releases/latest/download/archergate-license-linux-x64.tar.gz" style="font-family:Georgia,serif;font-size:15px;color:#222;text-decoration:underline;">Download .tar.gz</a>
                        </td></tr>
                        <tr><td style="padding:12px 0;">
                            <span style="font-family:'Courier New',monospace;font-size:12px;color:#888;">RUST (CRATES.IO)</span><br>
                            <span style="font-family:'Courier New',monospace;font-size:14px;color:#222;">cargo add archergate-license</span>
                        </td></tr>
                    </table>

                    <p style="font-family:Georgia,serif;font-size:16px;color:#444;line-height:1.7;margin:24px 0 20px;">
                        Three lines to validate a license:
                    </p>

                    <table width="100%" cellpadding="0" cellspacing="0">
                        <tr><td style="background:#222;border-radius:4px;padding:16px 20px;">
                            <code style="font-family:'Courier New',monospace;font-size:13px;color:#DEE1E8;line-height:1.6;">
                                #include "archergate_license.h"<br>
                                AgLicenseClient* c = ag_license_new("key", "${appId || 'com.you.app'}");<br>
                                ag_license_validate(c, license_key);
                            </code>
                        </td></tr>
                    </table>

                    <p style="font-family:Georgia,serif;font-size:16px;color:#444;line-height:1.7;margin:24px 0;">
                        Full docs and source:<br>
                        <a href="https://github.com/lailaarcher/archergate" style="color:#222;text-decoration:underline;">github.com/lailaarcher/archergate</a>
                    </p>

                    <!-- CTA -->
                    <table cellpadding="0" cellspacing="0" style="margin:32px 0;">
                        <tr><td style="background:#222;border-radius:4px;">
                            <a href="https://archergate.io" style="display:inline-block;padding:14px 32px;font-family:'Courier New',monospace;font-size:13px;letter-spacing:0.08em;color:#F5F3EF;text-decoration:none;">ARCHERGATE.IO &rarr;</a>
                        </td></tr>
                    </table>

                    <p style="font-family:Georgia,serif;font-size:16px;color:#444;line-height:1.7;margin:24px 0 0;">
                        While you're here: what is the one thing about licensing that has burned you before? Hit reply and tell us. That is how we decide what to build next.
                    </p>

                    <p style="font-family:Georgia,serif;font-size:16px;color:#444;line-height:1.7;margin:32px 0 0;">
                        Thanks for your support,<br>
                        Archergate Team
                    </p>
                </td></tr>

                <!-- Footer -->
                <tr><td style="padding:24px 0 0;border-top:1px solid #D4D0C8;">
                    <span style="font-family:Georgia,serif;font-size:14px;font-weight:bold;color:#222;">Archergate</span><br>
                    <span style="font-family:Georgia,serif;font-size:13px;color:#888;">
                        <a href="https://archergate.io" style="color:#888;text-decoration:none;">archergate.io</a> | 447 Sutter St, Ste 506 1461, San Francisco CA 94108
                    </span>
                </td></tr>

            </table>
        </td></tr>
    </table>
</body>
</html>`;
}
