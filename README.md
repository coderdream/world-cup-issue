# WorldCupIssue

WorldCupIssue锛堜笘鐣屾澂缁勬墜锛夋槸涓€涓?Windows 浼樺厛鐨?Tauri 2 妗岄潰搴旂敤锛岀洰鏍囨槸澶嶅埢鏉喌 CupWatch v0.1.3 鐨勪笘鐣屾澂妗岄潰鐩垎浣撻獙锛屽悓鏃朵繚鐣欐竻鏅扮殑妗嗘灦鍖栧垎灞傦紝渚夸簬缁х画鎵╁睍銆?
## 椤圭洰韬唤

- 椤圭洰鍚嶏細`WorldCupIssue锛堜笘鐣屾澂缁勬墜锛塦
- 鍙墽琛屾枃浠讹細`world_cup_issue.exe`
- 鐗堟湰鍙凤細`0.1.11`
- Tauri identifier / 鐢ㄦ埛鏁版嵁鐩綍锛歚com.worldcupissue.app`

## 鍒嗗眰

- `src/config`锛氳矾鐢便€佸鑸€侀〉闈㈠厓淇℃伅绛夊彲閰嶇疆鍏ュ彛銆?- `src/domain`锛氳禌浜嬨€侀娴嬨€佺Н鍒嗙瓑绾笟鍔¤绠椼€?- `src/lib/api`锛氬墠绔闂悗绔?IPC 鐨勫敮涓€闂ㄩ潰銆?- `src/store`锛歓ustand 搴旂敤鐘舵€侊紝璐熻矗缁勫悎 API 涓?UI銆?- `src/pages`锛氶〉闈㈢紪鎺掞紝灏介噺鍙秷璐?store銆乨omain 涓庣粍浠躲€?- `src-tauri/src/commands.rs`锛欼PC 鍛戒护灞傘€?- `src-tauri/src/database.rs`锛歋QLite 琛ㄧ粨鏋勫拰鏈湴鎸佷箙鍖栥€?- `docs`锛歏itePress 瀹樼綉/docs 绔欑偣銆?
## 寮€鍙?
```bash
pnpm install
pnpm dev
```

## 鏋勫缓

```bash
pnpm build
pnpm docs:build
pnpm tauri:build
```

褰撳墠 Windows 鏋勫缓浣跨敤 Tauri 2 NSIS 瀹夎鍖呫€傛瘡娆′慨鏀瑰悗闇€灏嗚ˉ涓佺増鏈彿澧炲姞 `0.0.1`锛岄噸鏂版墦鍖咃紝骞跺悓姝ョ敓鎴愭洿鏂?manifest銆?
```bash
pnpm tauri build --target x86_64-pc-windows-gnu
node scripts/write-update-manifest.mjs
```

褰撳墠浠撳簱鍦?Windows 涓婃帹鑽愪娇鐢ㄥ浐瀹氳剼鏈細

```powershell
.\scripts\package-windows.ps1
```

鑴氭湰浼氶€氳繃鏈満浠ｇ悊 `127.0.0.1:1080` 鏋勫缓锛屼娇鐢?`.tooling/updater/worldcupissue.key` 绛惧悕瀹夎鍖咃紝骞剁敓鎴?`release/<version>/latest.json`銆?
## 鏁版嵁绛栫暐

鍏紑鏁版嵁婧愪紭鍏堢骇鍥哄畾涓猴細

1. `https://pub-9d9e6c0cb6934fb0a0c505e3c64f39b2.r2.dev/cupwatch/data/worldcup-2026.json`
2. `https://cdn.jsdelivr.net/gh/openfootball/worldcup.json@master/2026/worldcup.json`
3. `https://raw.githubusercontent.com/openfootball/worldcup.json/master/2026/worldcup.json`

`football-data.org` Token 鏄彲閫夊閲忔簮锛屼粎鏈湴淇濆瓨銆?


