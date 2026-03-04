import type { ActivityItem, DownloadItem, ModPackage } from "./types";

export const seedPackages: ModPackage[] = [
  {
    id: "bepinex-pack",
    fullName: "BepInEx-BepInExPack",
    author: "BepInEx",
    summary:
      "Base loader and patching layer for Lethal Company, pinned around the versions that behave cleanly with v49.",
    categories: ["Core", "Loader", "Dependency"],
    totalDownloads: 2850021,
    rating: 4.9,
    websiteUrl: "https://thunderstore.io/c/lethal-company/p/BepInEx/BepInExPack/",
    versions: [
      {
        id: "bepinex-5417",
        versionNumber: "5.4.2100",
        publishedAt: "2024-01-20",
        downloads: 410112,
        baseZone: "green",
        bundledReferenceState: "verified",
        bundledReferenceNote:
          "Confirmed stable with core v49 stacks and 8-player sessions."
      },
      {
        id: "bepinex-5418",
        versionNumber: "5.4.2200",
        publishedAt: "2024-04-17",
        downloads: 901220,
        baseZone: "red"
      }
    ]
  },
  {
    id: "more-company",
    fullName: "notnotnotswipez-MoreCompany",
    author: "notnotnotswipez",
    summary:
      "Lobby expansion and lightweight quality-of-life changes for larger friend groups.",
    categories: ["Lobby", "Multiplayer", "QoL"],
    totalDownloads: 1912033,
    rating: 4.7,
    websiteUrl: "https://thunderstore.io/c/lethal-company/p/notnotnotswipez/MoreCompany/",
    versions: [
      {
        id: "more-176",
        versionNumber: "1.7.6",
        publishedAt: "2024-03-14",
        downloads: 240100,
        baseZone: "green",
        bundledReferenceState: "verified",
        bundledReferenceNote:
          "Confirmed working with BepInExPack 5.4.2100 and vanilla-hosted lobbies."
      },
      {
        id: "more-177",
        versionNumber: "1.7.7",
        publishedAt: "2024-04-03",
        downloads: 191221,
        baseZone: "yellow"
      },
      {
        id: "more-180",
        versionNumber: "1.8.0",
        publishedAt: "2024-04-14",
        downloads: 310113,
        baseZone: "red"
      }
    ]
  },
  {
    id: "lc-api",
    fullName: "2018-LC_API",
    author: "2018",
    summary:
      "A broad dependency surface for older Lethal Company mods, but some releases cause round-start desync on v49.",
    categories: ["API", "Dependency", "Core"],
    totalDownloads: 1220143,
    rating: 4.2,
    websiteUrl: "https://thunderstore.io/c/lethal-company/p/2018/LC_API/",
    versions: [
      {
        id: "lcapi-343",
        versionNumber: "3.4.3",
        publishedAt: "2024-01-15",
        downloads: 120001,
        baseZone: "green",
        bundledReferenceState: "verified",
        bundledReferenceNote: "Good baseline for older v49 dependency chains."
      },
      {
        id: "lcapi-345",
        versionNumber: "3.4.5",
        publishedAt: "2024-02-01",
        downloads: 184940,
        baseZone: "green",
        bundledReferenceState: "broken",
        bundledReferenceNote:
          "Known to trigger lobby desync after the first quota rollover."
      },
      {
        id: "lcapi-350",
        versionNumber: "3.5.0",
        publishedAt: "2024-04-16",
        downloads: 201114,
        baseZone: "red"
      }
    ]
  },
  {
    id: "mimics",
    fullName: "x753-Mimics",
    author: "x753",
    summary:
      "Creature encounter mod that adds extra tension, but older versions need scrutiny for v49 balance and compatibility.",
    categories: ["Enemies", "Content", "Immersion"],
    totalDownloads: 875000,
    rating: 4.6,
    websiteUrl: "https://thunderstore.io/c/lethal-company/p/x753/Mimics/",
    versions: [
      {
        id: "mimics-210",
        versionNumber: "2.1.0",
        publishedAt: "2023-11-24",
        downloads: 80011,
        baseZone: "orange"
      },
      {
        id: "mimics-220",
        versionNumber: "2.2.0",
        publishedAt: "2024-02-18",
        downloads: 122903,
        baseZone: "green",
        bundledReferenceState: "verified",
        bundledReferenceNote: "Confirmed playable on dedicated host and peer host."
      }
    ]
  },
  {
    id: "coilhead-stare",
    fullName: "Renegades-CoilHeadStare",
    author: "Renegades",
    summary:
      "A small enemy behavior tweak with a rough v49 history. Useful as a stress-test for broken-version flows.",
    categories: ["Enemies", "Challenge", "Experimental"],
    totalDownloads: 240800,
    rating: 3.9,
    websiteUrl: "https://thunderstore.io/c/lethal-company/p/Renegades/CoilHeadStare/",
    versions: [
      {
        id: "coil-091",
        versionNumber: "0.9.1",
        publishedAt: "2024-03-01",
        downloads: 40110,
        baseZone: "green",
        bundledReferenceState: "broken",
        bundledReferenceNote: "Confirmed null-ref on facility load for v49 clients."
      },
      {
        id: "coil-080",
        versionNumber: "0.8.0",
        publishedAt: "2024-01-10",
        downloads: 38112,
        baseZone: "green"
      }
    ]
  }
];

export const seedDownloads: DownloadItem[] = [
  {
    id: "dl-1",
    packageName: "BepInEx-BepInExPack",
    versionNumber: "5.4.2100",
    progressLabel: "Available in the global cache",
    status: "cached",
    speedLabel: "copied from cache",
    cacheHit: true
  },
  {
    id: "dl-2",
    packageName: "2018-LC_API",
    versionNumber: "3.4.3",
    progressLabel: "Waiting for first download into the global cache",
    status: "queued",
    speedLabel: "waiting",
    cacheHit: false
  }
];

export const seedActivities: ActivityItem[] = [
  {
    id: "activity-1",
    title: "v49 install validated",
    detail: "Depot-swapped executable matches the expected v49 signature and activation path checks passed.",
    tone: "positive"
  },
  {
    id: "activity-2",
    title: "Broken version tracking active",
    detail: "3 exact versions are marked broken locally and will trigger a download warning before install.",
    tone: "warning"
  },
  {
    id: "activity-3",
    title: "Catalog cache warm",
    detail: "The UI is currently browsing local metadata and can stay responsive even if Thunderstore is unreachable.",
    tone: "neutral"
  }
];
