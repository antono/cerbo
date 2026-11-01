// WebdriverIO -> tauri-driver -> WebKitWebDriver -> cerbo-desktop.
// See openspec/changes/add-desktop-e2e-tests/design.md.
import { spawn, type ChildProcess } from 'node:child_process';
import { SevereServiceError } from 'webdriverio';
import { createSession, desktopBinary, destroySession } from './session.ts';

let tauriDriver: ChildProcess | undefined;
let sessionRoot: string | undefined;

export const config: WebdriverIO.Config = {
  runner: 'local',
  hostname: '127.0.0.1',
  port: 4444,
  specs: ['./specs/**/*.e2e.ts'],
  // WebKitGTK allows one automation context at a time.
  maxInstances: 1,
  capabilities: [
    {
      // Replaced per session by the isolating launcher in beforeSession.
      // @ts-expect-error tauri-driver specific capability
      'tauri:options': { application: '' },
    },
  ],
  logLevel: 'warn',
  framework: 'mocha',
  reporters: ['spec'],
  mochaOpts: { ui: 'bdd', timeout: 60_000 },

  onPrepare() {
    try {
      desktopBinary();
    } catch (e) {
      // Only a SevereServiceError stops WebdriverIO before workers start.
      throw new SevereServiceError((e as Error).message);
    }
    const nativeDriver = process.env.WEBKIT_WEBDRIVER ?? 'WebKitWebDriver';
    tauriDriver = spawn('tauri-driver', ['--native-driver', nativeDriver], {
      stdio: [null, process.stdout, process.stderr],
    });
  },

  beforeSession(_config, capabilities, specs) {
    const session = createSession(specs);
    sessionRoot = session.root;
    // @ts-expect-error tauri-driver specific capability
    capabilities['tauri:options'] = { application: session.launcher };
  },

  afterSession() {
    if (sessionRoot) destroySession(sessionRoot);
  },

  onComplete() {
    tauriDriver?.kill();
  },
};
