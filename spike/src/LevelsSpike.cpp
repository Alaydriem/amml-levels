// THROWAWAY SPIKE. Proves (or disproves) flat, depth-tested light-level numbers drawn through the game's
// own name-tag list. Not part of the real mod. No tests, no Rust, one file on purpose.
//
// Keys (ignored while a screen is open):
//   L  overlay on/off            J  cycle rotation            M  cycle material
//   O  cycle scale               P  absolute / camera-relative position
//   B  background on/off         K  cycle label source (real scan, 500, 2000, 8000 synthetic)
//   H  inspect the looked-at block (solidity, collision, light)

#define LL_MEMORY_OPERATORS
#include "ll/api/memory/MemoryOperators.h" // IWYU pragma: keep

#include <ll/api/event/EventBus.h>
#include <ll/api/event/input/KeyInputEvent.h>
#include <ll/api/io/Logger.h>
#include <ll/api/memory/Hook.h>
#include <ll/api/mod/NativeMod.h>
#include <ll/api/mod/RegisterHelper.h>

#include <mc/client/game/ClientInstance.h>
#include <mc/client/gui/Font.h>
#include <mc/client/gui/FontHandle.h>
#include <mc/client/gui/screens/ScreenContext.h>
#include <mc/client/player/LocalPlayer.h>
#include <mc/client/renderer/BaseActorRenderer.h>
#include <mc/client/renderer/game/LevelRendererPlayer.h>
#include <mc/common/Brightness.h>
#include <mc/deps/core/math/Color.h>
#include <mc/deps/core/math/Quaternion.h>
#include <mc/deps/core/math/Vec2.h>
#include <mc/deps/core/math/Vec3.h>
#include <mc/deps/minecraft_renderer/objects/NameTagRenderObject.h>
#include <mc/deps/minecraft_renderer/objects/NameTagRenderObjectCollection.h>
#include <mc/deps/minecraft_renderer/objects/ViewRenderData.h>
#include <mc/deps/minecraft_renderer/objects/ViewRenderObject.h>
#include <mc/world/level/BlockPos.h>
#include <mc/world/level/BlockSource.h>
#include <mc/world/level/Level.h>
#include <mc/world/level/block/Block.h>
#include <mc/world/level/block/BlockType.h>
#include <mc/world/level/block/BrightnessPair.h>
#include <mc/world/level/dimension/Dimension.h>
#include <mc/world/phys/AABB.h>
#include <mc/world/phys/HitResult.h>

#include <glm/glm.hpp>
#include <glm/gtc/quaternion.hpp>

#include <windows.h>

#include <array>
#include <atomic>
#include <chrono>
#include <cmath>
#include <cstring>
#include <mutex>
#include <optional>
#include <string>
#include <vector>

namespace levels_spike {

namespace {

constexpr int   RADIUS        = 24;
constexpr int   MAX_REAL      = 8192;
constexpr int   FACE_UP       = 1;
constexpr int   VK_L          = 0x4C;
constexpr int   VK_J          = 0x4A;
constexpr int   VK_M          = 0x4D;
constexpr int   VK_O          = 0x4F;
constexpr int   VK_P          = 0x50;
constexpr int   VK_B          = 0x42;
constexpr int   VK_K          = 0x4B;
constexpr int   VK_H          = 0x48;
constexpr float LIFT          = 0.02f;
constexpr float SCAN_SECONDS  = 0.25f;
constexpr float REPORT_EVERY  = 5.0f;

constexpr std::array<float, 5> SCALES        = {0.0125f, 0.025f, 0.05f, 0.1f, 1.0f};
constexpr std::array<int, 4>   SOURCES       = {0, 500, 2000, 8000};
constexpr std::array<const char*, 4> MATERIALS = {
    "game default (no override)",
    "depth-tested tag + text",
    "depth-tested tag + text, with backface",
    "plain tag + text (not depth-tested)",
};

struct SpikeLabel {
    glm::vec3 pos;
    int       blockLight;
    int       skyLight;
};

// the in-memory shape of std::optional<Quaternion>: 16 bytes of quaternion, then the engaged flag
struct alignas(4) RawOptionalQuat {
    float q[4];
    bool  engaged;
    char  pad[3];
};
static_assert(sizeof(RawOptionalQuat) == 20);

ll::io::Logger*        g_log = nullptr;
ll::event::ListenerPtr g_keyListener;

std::mutex              g_mutex;
std::vector<SpikeLabel> g_labels;
std::atomic<ClientInstance*> g_client{nullptr};
std::atomic<bool>       g_on{false};
std::atomic<int>        g_rotation{0};
std::atomic<int>        g_material{1};
std::atomic<int>        g_scale{4};
std::atomic<bool>       g_relative{false};
std::atomic<bool>       g_background{false};
std::atomic<int>        g_source{0};
std::atomic<bool>       g_inspect{false};
glm::vec3               g_cameraPos{0.0f};

BaseActorRenderer*               g_textRenderer = nullptr;
std::array<std::vector<int>, 16> g_widths;
std::array<std::string, 16>      g_texts;
bool                             g_widthsReady = false;
// a ring of label arrays, one per frame in flight; the game destroys the elements, never the storage
std::array<std::vector<std::byte>, 8> g_pool;
size_t                                g_poolIndex = 0;

double nowSeconds() {
    static const double frequency = [] {
        LARGE_INTEGER value;
        QueryPerformanceFrequency(&value);
        return static_cast<double>(value.QuadPart);
    }();
    LARGE_INTEGER now;
    QueryPerformanceCounter(&now);
    return static_cast<double>(now.QuadPart) / frequency;
}

// flat (+90 degrees about X), turned about Y to follow the player's yaw: index 0..3 use +yaw, 4..7 use -yaw,
// each with an offset of 0 / 90 / 180 / 270 degrees
RawOptionalQuat rotationFor(int index, float yawDegrees) {
    RawOptionalQuat raw{};
    float     sign   = index < 4 ? 1.0f : -1.0f;
    float     offset = 90.0f * static_cast<float>(index % 4);
    float     x      = glm::radians(90.0f);
    float     y      = glm::radians(sign * yawDegrees + offset);
    glm::quat q      = glm::angleAxis(y, glm::vec3(0, 1, 0)) * glm::angleAxis(x, glm::vec3(1, 0, 0));
    std::memcpy(raw.q, &q, sizeof(raw.q));
    raw.engaged = true;
    return raw;
}

mce::Color colorFor(int blockLight, int) {
    if (blockLight == 0) return mce::Color(1.0f, 0.25f, 0.25f, 1.0f);
    if (blockLight <= 7) return mce::Color(1.0f, 0.85f, 0.2f, 1.0f);
    return mce::Color(0.35f, 1.0f, 0.35f, 1.0f);
}

// the light the game's spawn check sees at this time of day: the brighter of block light and darkened sky light
int timeLight(BlockSource const& region, BlockPos const& pos) {
    BrightnessPair light  = region.getBrightnessPair(pos);
    int            darken = static_cast<int>(region.getDimensionConst().mSkyDarken->mValue);
    int            sky    = std::max(0, static_cast<int>(light.sky->mValue) - darken);
    return std::max(static_cast<int>(light.block->mValue), sky);
}

bool passable(BlockSource const& region, Block const& block, BlockPos const& pos) {
    if (block.isAir()) return true;
    AABB shape = block.getBlockType().getCollisionShape(block, region, pos, {});
    return shape.min.x == shape.max.x && shape.min.y == shape.max.y && shape.min.z == shape.max.z;
}

void scanReal(LocalPlayer& player, std::vector<SpikeLabel>& out) {
    BlockSource const& region = player.getDimensionBlockSourceConst();
    BlockPos           feet   = player.getFeetBlockPos();
    int                yMin   = std::max<int>(feet.y - RADIUS, region.getMinHeight());
    int                yMax   = std::min<int>(feet.y + RADIUS, region.getMaxHeight() - 2);
    for (int dx = -RADIUS; dx <= RADIUS; ++dx) {
        for (int dz = -RADIUS; dz <= RADIUS; ++dz) {
            for (int y = yMin; y <= yMax; ++y) {
                int dy = y - feet.y;
                if (dx * dx + dy * dy + dz * dz > RADIUS * RADIUS) continue;
                BlockPos     pos{feet.x + dx, y, feet.z + dz};
                Block const& block = region.getBlock(pos);
                if (!block.canProvideFullSupport(static_cast<uchar>(FACE_UP))) continue;
                BlockPos     abovePos{pos.x, pos.y + 1, pos.z};
                Block const& above = region.getBlock(abovePos);
                if (!passable(region, above, abovePos)) continue;
                BrightnessPair light = region.getBrightnessPair(abovePos);
                out.push_back(SpikeLabel{
                    glm::vec3(pos.x + 0.5f, pos.y + 1.0f + LIFT, pos.z + 0.5f),
                    timeLight(region, abovePos),
                    static_cast<int>(light.sky->mValue),
                });
                if (static_cast<int>(out.size()) >= MAX_REAL) return;
            }
        }
    }
}

void scanSynthetic(LocalPlayer& player, int count, std::vector<SpikeLabel>& out) {
    BlockPos feet = player.getFeetBlockPos();
    int      side = static_cast<int>(std::ceil(std::sqrt(static_cast<float>(count))));
    for (int i = 0; i < count; ++i) {
        int gx = i % side - side / 2;
        int gz = i / side - side / 2;
        out.push_back(SpikeLabel{
            glm::vec3(feet.x + gx + 0.5f, feet.y + LIFT, feet.z + gz + 0.5f),
            i % 16,
            (i / 16) % 16,
        });
    }
}

void inspect(LocalPlayer& player) {
    HitResult& hit = player.getLevel().getHitResult();
    if (hit.mType != HitResultType::Tile) {
        g_log->info("[inspect] not looking at a block");
        return;
    }
    BlockSource const& region = player.getDimensionBlockSourceConst();
    BlockPos           pos    = hit.mBlock;
    BlockPos           up{pos.x, pos.y + 1, pos.z};
    Block const&       block  = region.getBlock(pos);
    Block const&       above  = region.getBlock(up);
    AABB               shape  = above.getBlockType().getCollisionShape(above, region, up, {});
    BrightnessPair     light  = region.getBrightnessPair(up);
    BrightnessPair     here   = region.getBrightnessPair(pos);
    g_log->info(
        "[inspect] {} at ({}, {}, {}): fullSupportUp={} supportUpCenter={} solid={} | above {}: air={} "
        "solid={} collision=({:.2f},{:.2f},{:.2f})-({:.2f},{:.2f},{:.2f}) | light above block={} sky={} | light "
        "in block block={} sky={} | sky darken {} getSkyDarken(1) {:.3f} | time light above {} | raw(no night) {} "
        "raw(night) {}",
        block.getTypeName(),
        pos.x,
        pos.y,
        pos.z,
        block.canProvideFullSupport(static_cast<uchar>(FACE_UP)),
        block.canProvideSupport(static_cast<uchar>(FACE_UP), BlockSupportType::Center),
        block._isSolid(),
        above.getTypeName(),
        above.isAir(),
        above._isSolid(),
        shape.min.x,
        shape.min.y,
        shape.min.z,
        shape.max.x,
        shape.max.y,
        shape.max.z,
        static_cast<int>(light.block->mValue),
        static_cast<int>(light.sky->mValue),
        static_cast<int>(here.block->mValue),
        static_cast<int>(here.sky->mValue),
        static_cast<int>(region.getDimensionConst().mSkyDarken->mValue),
        region.getDimensionConst().getSkyDarken(1.0f),
        timeLight(region, up),
        static_cast<int>(region._getRawBrightness(up, region.getDimensionConst().mSkyDarken, false).mValue),
        static_cast<int>(region._getRawBrightness(up, region.getDimensionConst().mSkyDarken, true).mValue)
    );
}

struct UpdateStats {
    double   windowStart = 0.0;
    double   lastScan    = 0.0;
    int      calls       = 0;
    double   scanMsTotal = 0.0;
    int      scans       = 0;
    size_t   lastCount   = 0;
};
UpdateStats g_update;

void updateTick(ClientInstance& client) {
    double now = nowSeconds();
    if (g_update.windowStart == 0.0) g_update.windowStart = now;
    g_update.calls++;
    LocalPlayer* player = client.getLocalPlayer();
    if (player && g_inspect.exchange(false)) inspect(*player);
    if (player && g_on.load() && now - g_update.lastScan >= SCAN_SECONDS) {
        g_update.lastScan = now;
        std::vector<SpikeLabel> fresh;
        fresh.reserve(MAX_REAL);
        double start  = nowSeconds();
        int    source = SOURCES[static_cast<size_t>(g_source.load())];
        if (source == 0) scanReal(*player, fresh);
        else scanSynthetic(*player, source, fresh);
        g_update.scanMsTotal += (nowSeconds() - start) * 1000.0;
        g_update.scans++;
        g_update.lastCount = fresh.size();
        std::lock_guard lock(g_mutex);
        g_labels.swap(fresh);
    }
    if (!g_on.load()) {
        std::lock_guard lock(g_mutex);
        g_labels.clear();
    }
    if (now - g_update.windowStart >= REPORT_EVERY) {
        g_log->info(
            "[update] thread {} calls/s {:.1f} scans {} avg scan {:.2f} ms labels {}",
            GetCurrentThreadId(),
            g_update.calls / (now - g_update.windowStart),
            g_update.scans,
            g_update.scans ? g_update.scanMsTotal / g_update.scans : 0.0,
            g_update.lastCount
        );
        g_update = UpdateStats{now, g_update.lastScan};
    }
}

struct DrawStats {
    double windowStart = 0.0;
    double lastCall    = 0.0;
    int    calls       = 0;
    double maxFrameMs  = 0.0;
    double appendMs    = 0.0;
    size_t appended    = 0;
    bool   loggedTags  = false;
};
DrawStats g_draw;

void prepareText(ClientInstance& client) {
    if (g_widthsReady) return;
    FontHandle handle = client.getFontHandle();
    Font&      font   = handle.getFont();
    for (int i = 0; i < 16; ++i) {
        g_texts[static_cast<size_t>(i)]  = std::to_string(i);
        g_widths[static_cast<size_t>(i)] = {font.getLineLength(g_texts[static_cast<size_t>(i)], 1.0f, false)};
    }
    if (!g_textRenderer) g_textRenderer = new BaseActorRenderer();
    g_widthsReady = true;
    g_log->info("[draw] text widths ready: '0'={} '15'={}", g_widths[0][0], g_widths[15][0]);
}

void materialsFor(int mode, mce::MaterialPtr const*& tag, mce::MaterialPtr const*& text) {
    switch (mode) {
    case 1:
        tag  = &*g_textRenderer->mDepthTestedNameTagMat;
        text = &*g_textRenderer->mDepthTestedNameTextMat;
        return;
    case 2:
        tag  = &*g_textRenderer->mDepthTestedNameTagWithBackfaceMat;
        text = &*g_textRenderer->mDepthTestedNameTextWithBackfaceMat;
        return;
    case 3:
        tag  = &*g_textRenderer->mNameTagBackgroundMat;
        text = &*g_textRenderer->mNameTagTextMat;
        return;
    default:
        tag  = nullptr;
        text = nullptr;
        return;
    }
}

void logExistingTags(NameTagRenderObjectCollection const& tags) {
    if (g_draw.loggedTags || tags.mNameTags->empty()) return;
    g_draw.loggedTags = true;
    ClientInstance* client = g_client.load();
    LocalPlayer*    player = client ? client->getLocalPlayer() : nullptr;
    Vec3            feet   = player ? player->getFeetPos() : Vec3{0, 0, 0};
    for (auto const& tag : *tags.mNameTags) {
        glm::vec3 pos = tag.mPos;
        g_log->info(
            "[tags] existing '{}' pos ({:.2f}, {:.2f}, {:.2f}) scale {} rotation {} | player feet ({:.2f}, {:.2f}, "
            "{:.2f}) camera ({:.2f}, {:.2f}, {:.2f})",
            *tag.mNameTag,
            pos.x,
            pos.y,
            pos.z,
            static_cast<float>(tag.mScale),
            tag.mCustomRotation->has_value(),
            feet.x,
            feet.y,
            feet.z,
            g_cameraPos.x,
            g_cameraPos.y,
            g_cameraPos.z
        );
    }
}

void appendLabels(ScreenContext& screen, NameTagRenderObjectCollection& tags) {
    ClientInstance* client = g_client.load();
    if (!client || !g_on.load()) return;
    prepareText(*client);
    mce::MaterialPtr const* tagMat  = nullptr;
    mce::MaterialPtr const* textMat = nullptr;
    materialsFor(g_material.load(), tagMat, textMat);
    LocalPlayer*    viewer   = client->getLocalPlayer();
    float           yaw      = viewer ? viewer->getRotation().y : 0.0f;
    RawOptionalQuat raw      = rotationFor(g_rotation.load(), yaw);
    auto const&     rotation = *reinterpret_cast<std::optional<Quaternion> const*>(&raw);
    float           scale    = SCALES[static_cast<size_t>(g_scale.load())];
    mce::Color      tagColor(0.0f, 0.0f, 0.0f, g_background.load() ? 0.4f : 0.0f);
    glm::vec3       offset   = g_relative.load() ? g_cameraPos : glm::vec3(0.0f);
    std::lock_guard lock(g_mutex);
    if (g_labels.empty()) return;
    // MSVC vector with a stateful allocator: 16 bytes of LinearAllocator, then first / last / end
    static_assert(sizeof(std::vector<NameTagRenderObject, LinearAllocator<NameTagRenderObject>>) == 40);
    auto*  raw2     = reinterpret_cast<char*>(&*tags.mNameTags);
    auto** first    = reinterpret_cast<NameTagRenderObject**>(raw2 + 16);
    auto** last     = reinterpret_cast<NameTagRenderObject**>(raw2 + 24);
    auto** end      = reinterpret_cast<NameTagRenderObject**>(raw2 + 32);
    size_t existing = static_cast<size_t>(*last - *first);
    size_t total    = existing + g_labels.size();
    auto&  slot     = g_pool[g_poolIndex];
    g_poolIndex     = (g_poolIndex + 1) % g_pool.size();
    if (slot.size() < total * sizeof(NameTagRenderObject)) slot.resize(total * sizeof(NameTagRenderObject));
    auto* storage = reinterpret_cast<NameTagRenderObject*>(slot.data());
    // relocate the game's own tags bit for bit; their old storage is never destroyed or freed (deallocate is a no-op)
    if (existing) std::memcpy(static_cast<void*>(storage), *first, existing * sizeof(NameTagRenderObject));
    NameTagRenderObject* cursor = storage + existing;
    for (auto const& label : g_labels) {
        size_t              index = static_cast<size_t>(std::clamp(label.blockLight, 0, 15));
        glm::vec3           at    = label.pos - offset;
        NameTagRenderObject made  = BaseActorRenderer::_extractRenderTextObject(
            screen.tessellator,
            g_texts[index],
            g_widths[index],
            Vec3{at.x, at.y, at.z},
            colorFor(label.blockLight, label.skyLight),
            tagMat,
            textMat,
            tagColor,
            scale,
            rotation,
            0.0f
        );
        new (cursor) NameTagRenderObject(made);
        ++cursor;
    }
    *first          = storage;
    *last           = cursor;
    *end            = cursor;
    g_draw.appended = g_labels.size();
}

void drawStats(double appendMs) {
    double now = nowSeconds();
    if (g_draw.windowStart == 0.0) g_draw.windowStart = now;
    if (g_draw.lastCall != 0.0) g_draw.maxFrameMs = std::max(g_draw.maxFrameMs, (now - g_draw.lastCall) * 1000.0);
    g_draw.lastCall = now;
    g_draw.calls++;
    g_draw.appendMs += appendMs;
    double window = now - g_draw.windowStart;
    if (window < REPORT_EVERY) return;
    g_log->info(
        "[draw] thread {} fps {:.1f} avg frame {:.2f} ms max frame {:.2f} ms | labels {} avg append {:.3f} ms | "
        "source {} rotation {} material '{}' scale {} relative {} background {}",
        GetCurrentThreadId(),
        g_draw.calls / window,
        window * 1000.0 / g_draw.calls,
        g_draw.maxFrameMs,
        g_draw.appended,
        g_draw.appendMs / g_draw.calls,
        SOURCES[static_cast<size_t>(g_source.load())],
        g_rotation.load(),
        MATERIALS[static_cast<size_t>(g_material.load())],
        SCALES[static_cast<size_t>(g_scale.load())],
        g_relative.load(),
        g_background.load()
    );
    bool logged = g_draw.loggedTags;
    g_draw      = DrawStats{now, now};
    g_draw.loggedTags = logged;
}

} // namespace

LL_TYPE_INSTANCE_HOOK(
    ClientUpdateHook,
    ll::memory::HookPriority::Normal,
    ClientInstance,
    &ClientInstance::$update,
    bool,
    bool a1
) {
    g_client.store(this);
    bool result = origin(a1);
    updateTick(*this);
    return result;
}

LL_TYPE_INSTANCE_HOOK(
    ViewObjectHook,
    ll::memory::HookPriority::Normal,
    LevelRendererPlayer,
    &LevelRendererPlayer::createViewRenderObject,
    ViewRenderObject,
    ScreenContext& screenContext,
    SubClientId    clientSubId
) {
    ViewRenderObject result = origin(screenContext, clientSubId);
    g_cameraPos             = result.mViewData->mCameraPos;
    return result;
}

LL_TYPE_INSTANCE_HOOK(
    NameTagHook,
    ll::memory::HookPriority::Normal,
    LevelRendererPlayer,
    &LevelRendererPlayer::$extractNameTags,
    NameTagRenderObjectCollection,
    ScreenContext& screenContext
) {
    NameTagRenderObjectCollection tags  = origin(screenContext);
    logExistingTags(tags);
    double                        start = nowSeconds();
    appendLabels(screenContext, tags);
    drawStats((nowSeconds() - start) * 1000.0);
    return tags;
}

template <typename T, size_t N>
int cycle(std::atomic<int>& value, std::array<T, N> const&) {
    int next = (value.load() + 1) % static_cast<int>(N);
    value.store(next);
    return next;
}

bool onKey(int key) {
    switch (key) {
    case VK_L:
        g_on.store(!g_on.load());
        g_log->info("overlay {}", g_on.load() ? "on" : "off");
        return true;
    case VK_J: {
        int next = (g_rotation.load() + 1) % 8;
        g_rotation.store(next);
        g_log->info("rotation {}", next);
        return true;
    }
    case VK_M:
        g_log->info("material '{}'", MATERIALS[static_cast<size_t>(cycle(g_material, MATERIALS))]);
        return true;
    case VK_O:
        g_log->info("scale {}", SCALES[static_cast<size_t>(cycle(g_scale, SCALES))]);
        return true;
    case VK_P:
        g_relative.store(!g_relative.load());
        g_log->info("position {}", g_relative.load() ? "camera-relative" : "absolute");
        return true;
    case VK_B:
        g_background.store(!g_background.load());
        g_log->info("background {}", g_background.load() ? "on" : "off");
        return true;
    case VK_K:
        g_log->info("label source {}", SOURCES[static_cast<size_t>(cycle(g_source, SOURCES))]);
        return true;
    case VK_H:
        g_inspect.store(true);
        return true;
    default:
        return false;
    }
}

class SpikeMod {
public:
    static SpikeMod& getInstance() {
        static SpikeMod instance;
        return instance;
    }

    SpikeMod() : mSelf(*ll::mod::NativeMod::current()) {}

    [[nodiscard]] ll::mod::NativeMod& getSelf() const { return mSelf; }

    bool load() { return true; }

    bool enable() {
        g_log = &mSelf.getLogger();
        ClientUpdateHook::hook();
        ViewObjectHook::hook();
        NameTagHook::hook();
        auto& bus     = ll::event::EventBus::getInstance();
        g_keyListener = bus.emplaceListener<ll::event::KeyInputEvent>([](ll::event::KeyInputEvent& event) {
            if (!event.isDown()) return;
            ClientInstance* client = g_client.load();
            if (client && client->isShowingMenu()) return;
            if (onKey(event.keyCode())) event.cancel();
        });
        g_log->info("levels spike loaded: L on/off, J rotation, M material, O scale, P position, B background, K "
                    "source, H inspect");
        return true;
    }

    bool disable() {
        auto& bus = ll::event::EventBus::getInstance();
        if (g_keyListener) bus.removeListener<ll::event::KeyInputEvent>(g_keyListener);
        g_keyListener = nullptr;
        NameTagHook::unhook();
        ViewObjectHook::unhook();
        ClientUpdateHook::unhook();
        return true;
    }

private:
    ll::mod::NativeMod& mSelf;
};

} // namespace levels_spike

LL_REGISTER_MOD(levels_spike::SpikeMod, levels_spike::SpikeMod::getInstance());
