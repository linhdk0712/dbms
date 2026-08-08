<script>
  import { onMount, onDestroy, tick } from "svelte";
  import { authStore } from "../stores/auth.js";

  let activeSection = "intro";
  let searchQuery = "";
  let observer = null;
  let mainEl = null;

  const sections = [
    { id: "intro", title: "Giới thiệu", icon: "📋" },
    { id: "login", title: "Đăng nhập & Xác thực", icon: "🔐" },
    { id: "ui", title: "Giao diện tổng quan", icon: "🖥️" },
    { id: "dashboard", title: "Dashboard", icon: "📊" },
    { id: "maker", title: "Hướng dẫn cho Maker", icon: "✏️" },
    { id: "checker", title: "Hướng dẫn cho Checker", icon: "✅" },
    { id: "data-query", title: "Data Query", icon: "🔍" },
    { id: "admin", title: "Hướng dẫn cho Admin", icon: "⚙️" },
    { id: "cr", title: "Change Requests", icon: "🔄" },
    { id: "notifications", title: "Thông báo Realtime", icon: "🔔" },
    { id: "ai", title: "Phân tích SQL (AI)", icon: "🤖" },
    { id: "rules", title: "Quy tắc nghiệp vụ", icon: "📜" },
    { id: "faq", title: "FAQ", icon: "❓" },
  ];

  $: filteredSections = searchQuery
    ? sections.filter(
        (s) =>
          s.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
          s.id.includes(searchQuery.toLowerCase()),
      )
    : sections;

  // Track which sections are currently visible in viewport
  let visibleSections = new Set();

  onMount(async () => {
    await tick();
    setupObserver();
  });

  onDestroy(() => {
    if (observer) observer.disconnect();
  });

  /**
   * Sets up an IntersectionObserver on the main content area to track which
   * manual sections are visible in the viewport, automatically updating the
   * active sidebar item as the user scrolls.
   */
  function setupObserver() {
    if (!mainEl) return;

    observer = new IntersectionObserver(
      (entries) => {
        entries.forEach((entry) => {
          const id = entry.target.id.replace("section-", "");
          if (entry.isIntersecting) {
            visibleSections.add(id);
          } else {
            visibleSections.delete(id);
          }
        });

        // Pick the first visible section (by document order)
        for (const section of sections) {
          if (visibleSections.has(section.id)) {
            activeSection = section.id;
            break;
          }
        }
      },
      {
        root: mainEl,
        rootMargin: "-10% 0px -80% 0px",
        threshold: 0,
      },
    );

    // Observe all section elements
    sections.forEach((s) => {
      const el = document.getElementById(`section-${s.id}`);
      if (el) observer.observe(el);
    });
  }

  /**
   * Scrolls the main content area to the specified manual section and marks it
   * as the currently active section in the sidebar.
   * @param {string} id - The section identifier to scroll to (e.g. "intro", "maker").
   */
  function scrollToSection(id) {
    activeSection = id;
    const el = document.getElementById(`section-${id}`);
    if (el) el.scrollIntoView({ behavior: "smooth", block: "start" });
  }
</script>

<div class="flex h-[calc(100vh-4rem)]">
  <!-- Sidebar TOC -->
  <aside
    class="w-72 border-r border-base-300 bg-base-100 overflow-y-auto p-4 hidden md:block"
  >
    <h3 class="font-bold text-lg mb-3">📖 Hướng dẫn sử dụng</h3>

    <input
      type="text"
      placeholder="Tìm kiếm..."
      class="input input-sm input-bordered w-full mb-3"
      bind:value={searchQuery}
    />

    <ul class="menu menu-sm gap-0.5">
      {#each filteredSections as section}
        <li>
          <button
            class:active={activeSection === section.id}
            on:click={() => scrollToSection(section.id)}
            class="text-left"
          >
            <span>{section.icon}</span>
            <span>{section.title}</span>
          </button>
        </li>
      {/each}
    </ul>
  </aside>

  <!-- Main content -->
  <main class="flex-1 overflow-y-auto p-6 prose prose-sm max-w-none scroll-pt-4" bind:this={mainEl}>
    <!-- Section: Intro -->
    <section id="section-intro">
      <h1 class="text-3xl font-bold mb-2">
        Hướng dẫn sử dụng SECS
      </h1>
      <p class="text-base-content/70 mb-6">
        SQL Execution Control System — Hệ thống quản lý và kiểm soát thực thi
        SQL trên môi trường production.
      </p>

      <div class="alert alert-info mb-6">
        <svg
          xmlns="http://www.w3.org/2000/svg"
          class="h-5 w-5"
          fill="none"
          viewBox="0 0 24 24"
          stroke="currentColor"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
          />
        </svg>
        <span
          >SECS áp dụng quy trình <strong>Maker-Checker</strong> (4-eyes
          principle) để đảm bảo mọi thao tác SQL trên database production đều
          được review và phê duyệt.</span
        >
      </div>

      <h2>Đối tượng sử dụng</h2>
      <div class="overflow-x-auto">
        <table class="table table-sm table-zebra">
          <thead>
            <tr>
              <th>Role</th>
              <th>Mô tả</th>
              <th>Quyền hạn chính</th>
            </tr>
          </thead>
          <tbody>
            <tr>
              <td><span class="badge badge-primary">Admin</span></td>
              <td>Quản trị viên hệ thống</td>
              <td>Quản lý users, databases, cấu hình hệ thống</td>
            </tr>
            <tr>
              <td><span class="badge badge-secondary">Maker</span></td>
              <td>Nhân viên vận hành</td>
              <td>Soạn và gửi SQL request để duyệt</td>
            </tr>
            <tr>
              <td><span class="badge badge-accent">Checker</span></td>
              <td>Quản lý / trưởng nhóm</td>
              <td>Review và phê duyệt SQL request</td>
            </tr>
            <tr>
              <td><span class="badge badge-warning">Approver</span></td>
              <td>Người phê duyệt Data Query</td>
              <td>Quản lý approval rules, phê duyệt data queries</td>
            </tr>
          </tbody>
        </table>
      </div>
      <p class="text-sm text-base-content/60 mt-2">
        💡 Một user có thể có nhiều role đồng thời.
      </p>

      <h2>Database hỗ trợ</h2>
      <div class="grid grid-cols-2 md:grid-cols-4 gap-3 not-prose">
        <div class="card bg-base-200 p-3 text-center">
          <div class="text-2xl mb-1">🐘</div>
          <div class="font-semibold text-sm">PostgreSQL</div>
          <div class="text-xs text-base-content/60">Đầy đủ</div>
        </div>
        <div class="card bg-base-200 p-3 text-center">
          <div class="text-2xl mb-1">🐬</div>
          <div class="font-semibold text-sm">MySQL</div>
          <div class="text-xs text-base-content/60">Đầy đủ</div>
        </div>
        <div class="card bg-base-200 p-3 text-center">
          <div class="text-2xl mb-1">🏛️</div>
          <div class="font-semibold text-sm">Oracle</div>
          <div class="text-xs text-base-content/60">Execute</div>
        </div>
        <div class="card bg-base-200 p-3 text-center">
          <div class="text-2xl mb-1">🍃</div>
          <div class="font-semibold text-sm">MongoDB</div>
          <div class="text-xs text-base-content/60">JSON commands</div>
        </div>
      </div>
    </section>

    <!-- Section: Login -->
    <section id="section-login" class="mt-12">
      <h1>🔐 Đăng nhập & Xác thực</h1>

      <h2>Đăng nhập qua SSO</h2>
      <p>
        SECS sử dụng <strong>Single Sign-On (SSO)</strong> qua giao thức
        OAuth2/OpenID Connect, tích hợp với hệ thống xác thực nội bộ của công ty.
      </p>

      <div class="steps steps-vertical lg:steps-horizontal w-full not-prose my-4">
        <div class="step step-primary">Truy cập SECS</div>
        <div class="step step-primary">Nhấn "Đăng nhập SSO"</div>
        <div class="step step-primary">Xác thực tại SSO</div>
        <div class="step step-primary">Tự động quay lại</div>
      </div>

      <h2>Phiên làm việc</h2>
      <ul>
        <li>Thời hạn phiên tối đa: <strong>8 giờ</strong></li>
        <li>Sau khi hết hạn, cần đăng nhập lại</li>
        <li>Đóng tab không mất session (token lưu trong browser)</li>
        <li>Nhấn <strong>"Đăng xuất"</strong> ở Navbar để kết thúc phiên</li>
      </ul>

      <div class="alert alert-warning mt-4">
        <span>⚡ <strong>Tài khoản mới</strong>: User SSO đầu tiên đăng nhập vào hệ thống sẽ tự động được gán role Admin. Các user sau cần Admin gán role.</span>
      </div>
    </section>

    <!-- Section: UI -->
    <section id="section-ui" class="mt-12">
      <h1>🖥️ Giao diện tổng quan</h1>

      <h2>Layout chính</h2>
      <div class="mockup-window bg-base-300 not-prose mb-4">
        <div class="bg-base-200 p-4">
          <div class="border-b border-base-300 pb-2 mb-2 font-bold text-sm">
            NAVBAR — Logo | Theme Toggle | User Menu
          </div>
          <div class="flex gap-2">
            <div class="w-48 bg-base-100 p-2 rounded text-xs">
              <p class="font-bold mb-1">SIDEBAR</p>
              <p>• Dashboard</p>
              <p>• Tạo request</p>
              <p>• Requests của tôi</p>
              <p>• Chờ duyệt</p>
              <p>• Data Queries</p>
              <p>• Admin...</p>
            </div>
            <div class="flex-1 bg-base-100 p-2 rounded text-xs">
              <p class="font-bold">NỘI DUNG CHÍNH</p>
              <p class="text-base-content/60">Hiển thị trang tương ứng với menu đang chọn</p>
            </div>
          </div>
        </div>
      </div>

      <h2>Menu hiển thị theo Role</h2>
      <div class="overflow-x-auto">
        <table class="table table-sm table-zebra">
          <thead>
            <tr><th>Mục menu</th><th>Role</th><th>Chức năng</th></tr>
          </thead>
          <tbody>
            <tr><td>Dashboard</td><td>Tất cả</td><td>Trang tổng quan</td></tr>
            <tr><td>Tạo request mới</td><td>Maker</td><td>Form tạo SQL request</td></tr>
            <tr><td>Requests của tôi</td><td>Maker</td><td>Danh sách request đã tạo</td></tr>
            <tr><td>Change Requests</td><td>Maker</td><td>Quản lý CR từ ManageEngine</td></tr>
            <tr><td>Chờ duyệt</td><td>Checker</td><td>Requests chờ approve</td></tr>
            <tr><td>Tạo Query mới</td><td>Tất cả</td><td>Form tạo data query</td></tr>
            <tr><td>Data Queries</td><td>Tất cả</td><td>Danh sách data queries</td></tr>
            <tr><td>Phê duyệt Query</td><td>Tất cả</td><td>Queries chờ duyệt</td></tr>
            <tr><td>Databases</td><td>Admin</td><td>Quản lý kết nối DB</td></tr>
            <tr><td>Users</td><td>Admin</td><td>Quản lý người dùng</td></tr>
            <tr><td>Approval Matrix</td><td>Admin/Approver</td><td>Cấu hình rules</td></tr>
            <tr><td>Approver Units</td><td>Admin/Approver</td><td>Quản lý đơn vị duyệt</td></tr>
          </tbody>
        </table>
      </div>
    </section>

    <!-- Section: Dashboard -->
    <section id="section-dashboard" class="mt-12">
      <h1>📊 Dashboard</h1>
      <p>Dashboard hiển thị thống kê tùy theo role của bạn:</p>

      <div class="grid grid-cols-1 md:grid-cols-3 gap-4 not-prose mt-4">
        <div class="card bg-primary/10 border border-primary/20 p-4">
          <h3 class="font-bold text-primary mb-2">Admin</h3>
          <ul class="text-sm space-y-1">
            <li>• Tổng số requests toàn hệ thống</li>
            <li>• Pending / Approved / Executed</li>
            <li>• Failed / Rejected / Cancelled</li>
          </ul>
        </div>
        <div class="card bg-accent/10 border border-accent/20 p-4">
          <h3 class="font-bold text-accent mb-2">Checker</h3>
          <ul class="text-sm space-y-1">
            <li>• Requests đang chờ review</li>
            <li>• Tổng đã review</li>
            <li>• Số đã approve / reject</li>
          </ul>
        </div>
        <div class="card bg-secondary/10 border border-secondary/20 p-4">
          <h3 class="font-bold text-secondary mb-2">Maker</h3>
          <ul class="text-sm space-y-1">
            <li>• Requests của bản thân</li>
            <li>• Phân theo trạng thái</li>
            <li>• Quick links tạo request</li>
          </ul>
        </div>
      </div>
    </section>

    <!-- Section: Maker -->
    <section id="section-maker" class="mt-12">
      <h1>✏️ Hướng dẫn cho Maker</h1>

      <h2>Tạo SQL Request mới</h2>
      <p><strong>Đường dẫn</strong>: Sidebar → "Tạo request mới"</p>

      <div class="not-prose">
        <ul class="steps steps-vertical my-4">
          <li class="step step-primary">
            <div class="text-left ml-4">
              <p class="font-bold">Chọn Database đích</p>
              <p class="text-sm text-base-content/60">Dropdown hiển thị databases bạn có quyền</p>
            </div>
          </li>
          <li class="step step-primary">
            <div class="text-left ml-4">
              <p class="font-bold">Nhập mô tả</p>
              <p class="text-sm text-base-content/60">Mục đích của SQL request (1-500 ký tự)</p>
            </div>
          </li>
          <li class="step step-primary">
            <div class="text-left ml-4">
              <p class="font-bold">Nhập nội dung SQL</p>
              <p class="text-sm text-base-content/60">SQL Editor có syntax highlighting (1-50,000 ký tự)</p>
            </div>
          </li>
          <li class="step step-primary">
            <div class="text-left ml-4">
              <p class="font-bold">Liên kết Change Request (tùy chọn)</p>
              <p class="text-sm text-base-content/60">Dán URL từ ManageEngine</p>
            </div>
          </li>
          <li class="step step-primary">
            <div class="text-left ml-4">
              <p class="font-bold">Lưu nháp hoặc Gửi duyệt</p>
              <p class="text-sm text-base-content/60">Draft = chỉnh sửa sau | Submit = gửi Checker review</p>
            </div>
          </li>
        </ul>
      </div>

      <h2>Vòng đời Request</h2>
      <div class="not-prose my-4">
        <div class="flex flex-wrap gap-2 items-center justify-center">
          <span class="badge badge-lg badge-ghost">DRAFT</span>
          <span>→</span>
          <span class="badge badge-lg badge-warning">PENDING</span>
          <span>→</span>
          <span class="badge badge-lg badge-success">APPROVED</span>
          <span>→</span>
          <span class="badge badge-lg badge-info">EXECUTED</span>
        </div>
        <div class="flex flex-wrap gap-2 items-center justify-center mt-2 text-sm text-base-content/60">
          <span>Hoặc: PENDING →</span>
          <span class="badge badge-sm badge-error">REJECTED</span>
          <span>/</span>
          <span class="badge badge-sm">CANCELLED</span>
          <span>/</span>
          <span class="badge badge-sm badge-error">FAILED</span>
        </div>
      </div>

      <h2>Các thao tác trên Request</h2>
      <div class="overflow-x-auto">
        <table class="table table-sm table-zebra">
          <thead>
            <tr><th>Thao tác</th><th>Khi nào</th><th>Mô tả</th></tr>
          </thead>
          <tbody>
            <tr><td><kbd class="kbd kbd-sm">Sửa</kbd></td><td>DRAFT</td><td>Chỉnh sửa mô tả và SQL</td></tr>
            <tr><td><kbd class="kbd kbd-sm">Submit</kbd></td><td>DRAFT</td><td>Gửi cho Checker → PENDING</td></tr>
            <tr><td><kbd class="kbd kbd-sm">Cancel</kbd></td><td>PENDING</td><td>Hủy request → CANCELLED</td></tr>
            <tr><td><kbd class="kbd kbd-sm">Clone</kbd></td><td>Bất kỳ</td><td>Tạo bản sao DRAFT mới</td></tr>
            <tr><td><kbd class="kbd kbd-sm">AI Analysis</kbd></td><td>DRAFT, PENDING</td><td>Phân tích SQL bằng AI</td></tr>
          </tbody>
        </table>
      </div>

      <div class="alert alert-error mt-4">
        <span>⚠️ <strong>Lưu ý quan trọng:</strong></span>
        <ul class="list-disc ml-4 mt-1">
          <li>Không thể tự approve request của mình (4-eyes principle)</li>
          <li>Không thể sửa SQL sau khi submit — phải Cancel rồi Clone</li>
          <li>Chỉ thấy request của mình — không xem được của người khác</li>
        </ul>
      </div>
    </section>

    <!-- Section: Checker -->
    <section id="section-checker" class="mt-12">
      <h1>✅ Hướng dẫn cho Checker</h1>

      <h2>Danh sách chờ duyệt</h2>
      <p><strong>Đường dẫn</strong>: Sidebar → "Chờ duyệt"</p>
      <ul>
        <li>Hiển thị requests có trạng thái PENDING</li>
        <li><strong>Không hiển thị</strong> request do chính bạn tạo (4-eyes)</li>
        <li>Thông tin: mô tả, database, người tạo, tiến trình approval</li>
      </ul>

      <h2>Review chi tiết</h2>
      <p>Nhấn vào request để xem đầy đủ:</p>
      <ul>
        <li><strong>Nội dung SQL</strong> — editor có syntax highlighting</li>
        <li><strong>Database info</strong> — loại DB, host, schema</li>
        <li><strong>DDL Analysis</strong> — cảnh báo tự động:
          <ul>
            <li><span class="badge badge-error badge-sm">Danger</span> DROP, TRUNCATE</li>
            <li><span class="badge badge-warning badge-sm">Warning</span> ALTER, CREATE</li>
          </ul>
        </li>
        <li><strong>Tiến trình approval</strong> — x/y levels đã approve</li>
      </ul>

      <h2>Hành động</h2>
      <div class="grid grid-cols-1 md:grid-cols-3 gap-4 not-prose mt-4">
        <div class="card bg-info/10 border border-info/20 p-4">
          <h4 class="font-bold text-info mb-2">🔎 Validate</h4>
          <p class="text-sm">Chạy EXPLAIN kiểm tra SQL hợp lệ mà không thực thi</p>
        </div>
        <div class="card bg-success/10 border border-success/20 p-4">
          <h4 class="font-bold text-success mb-2">✅ Approve</h4>
          <p class="text-sm">Phê duyệt. Khi đủ levels → SQL tự động thực thi</p>
        </div>
        <div class="card bg-error/10 border border-error/20 p-4">
          <h4 class="font-bold text-error mb-2">❌ Reject</h4>
          <p class="text-sm">Từ chối (bắt buộc nhập lý do, tối thiểu 10 ký tự)</p>
        </div>
      </div>

      <h2>Multi-Level Approval</h2>
      <p>Một request có thể yêu cầu nhiều cấp phê duyệt:</p>
      <ul>
        <li>Hiển thị tiến trình: "1/2 approvals" hoặc "2/2 approvals"</li>
        <li>Mỗi Checker chỉ approve <strong>1 lần</strong> cho mỗi request</li>
        <li>SQL chỉ thực thi khi <strong>đủ tất cả</strong> approval levels</li>
        <li>Kết quả: rows affected (thành công) hoặc error message (thất bại)</li>
      </ul>
    </section>

    <!-- Section: Data Query -->
    <section id="section-data-query" class="mt-12">
      <h1>🔍 Data Query — Truy vấn dữ liệu</h1>

      <div class="alert alert-info mb-4">
        <span>Data Query dành cho câu lệnh <strong>SELECT</strong> (read-only), với quy trình phê duyệt riêng phù hợp cho trích xuất dữ liệu, báo cáo.</span>
      </div>

      <h2>Tạo Data Query</h2>
      <p><strong>Đường dẫn</strong>: Sidebar → "Tạo Query mới"</p>

      <div class="grid grid-cols-1 md:grid-cols-2 gap-4 not-prose mt-4">
        <div class="card bg-base-200 p-4">
          <h4 class="font-bold mb-2">🖱️ Basic Mode</h4>
          <ul class="text-sm space-y-1">
            <li>• Chọn table từ schema browser</li>
            <li>• Tick columns cần lấy</li>
            <li>• Thêm điều kiện lọc (filters)</li>
            <li>• Hệ thống tự generate SQL</li>
          </ul>
        </div>
        <div class="card bg-base-200 p-4">
          <h4 class="font-bold mb-2">⌨️ Advanced Mode</h4>
          <ul class="text-sm space-y-1">
            <li>• Viết SQL SELECT tự do</li>
            <li>• Full SQL editor với autocomplete</li>
            <li>• Giới hạn: 5-50,000 ký tự</li>
            <li>• Phù hợp cho queries phức tạp</li>
          </ul>
        </div>
      </div>

      <h2>Thông tin cần điền</h2>
      <ul>
        <li><strong>Tiêu đề</strong> — Mục đích truy vấn (3-500 ký tự)</li>
        <li><strong>Database & Schema</strong> — Chọn từ danh sách</li>
        <li><strong>Ticket ID</strong> — Mã ticket liên quan (JIRA, etc.)</li>
        <li><strong>Assigned Users</strong> (tùy chọn) — Ủy quyền thực thi cho người khác</li>
        <li><strong>Ngày hết hạn</strong> (tùy chọn) — Sau ngày này không thể execute</li>
      </ul>

      <h2>Quy trình phê duyệt</h2>
      <div class="not-prose my-4">
        <div class="flex flex-wrap gap-2 items-center justify-center">
          <span class="badge badge-lg badge-ghost">DRAFT</span>
          <span>→</span>
          <span class="badge badge-lg badge-warning">PENDING APPROVAL</span>
          <span>→</span>
          <span class="badge badge-lg badge-success">APPROVED</span>
          <span>→</span>
          <span class="badge badge-lg badge-info">Execute (nhiều lần)</span>
        </div>
      </div>

      <h2>Thực thi Query</h2>
      <p>Sau khi APPROVED, nhấn <strong>"Execute"</strong> để chạy:</p>
      <ul>
        <li>Kết quả hiển thị dạng bảng (columns + rows)</li>
        <li>Tối đa <strong>10,000 rows</strong> (cảnh báo nếu bị cắt)</li>
        <li>Hiển thị thời gian thực thi</li>
        <li>Có thể execute nhiều lần cho đến khi hết hạn</li>
        <li>Mỗi lần execute đều được ghi audit</li>
      </ul>
    </section>

    <!-- Section: Admin -->
    <section id="section-admin" class="mt-12">
      <h1>⚙️ Hướng dẫn cho Admin</h1>

      <h2>Quản lý Database Connections</h2>
      <p><strong>Đường dẫn</strong>: Sidebar → "Databases"</p>

      <h3>Tạo connection mới</h3>
      <div class="overflow-x-auto">
        <table class="table table-sm">
          <thead><tr><th>Trường</th><th>Mô tả</th><th>Ví dụ</th></tr></thead>
          <tbody>
            <tr><td>Tên</td><td>Tên hiển thị</td><td>PROD - Oracle CRM</td></tr>
            <tr><td>Loại DB</td><td>postgresql / mysql / oracle / mongodb</td><td>oracle</td></tr>
            <tr><td>Host</td><td>Hostname hoặc IP</td><td>db.company.internal</td></tr>
            <tr><td>Port</td><td>Port kết nối</td><td>1521</td></tr>
            <tr><td>Database</td><td>Tên database</td><td>PRODDB</td></tr>
            <tr><td>Schema</td><td>Schema mặc định (tùy chọn)</td><td>HR_SCHEMA</td></tr>
            <tr><td>Username</td><td>Tài khoản DB</td><td>secs_app</td></tr>
            <tr><td>Password</td><td>Mật khẩu DB</td><td>••••••••</td></tr>
            <tr><td>Timeout</td><td>Thời gian chờ max (giây)</td><td>30</td></tr>
          </tbody>
        </table>
      </div>

      <div class="alert mt-4">
        <span>🔒 Credentials được mã hóa <strong>AES-256-GCM</strong> trước khi lưu. Không ai có thể xem lại password — chỉ có thể thay đổi.</span>
      </div>

      <h3>Thao tác quản lý</h3>
      <ul>
        <li><strong>Test Connection</strong> — Kiểm tra kết nối hoạt động</li>
        <li><strong>Test New</strong> — Test trước khi lưu (form tạo mới)</li>
        <li><strong>Đổi Credentials</strong> — Rotate password (có audit log)</li>
        <li><strong>Vô hiệu hóa</strong> — Không cho tạo request mới lên DB này</li>
      </ul>

      <h2>Quản lý Users</h2>
      <p><strong>Đường dẫn</strong>: Sidebar → "Users"</p>
      <ul>
        <li><strong>Gán Role</strong>: Admin, Maker, Checker, Approver</li>
        <li><strong>Cấp quyền Database</strong>: Chỉ định user → database cụ thể</li>
        <li><strong>Vô hiệu hóa</strong>: User inactive không thể đăng nhập</li>
        <li><strong>Gán Approver Unit</strong>: Thêm user vào nhóm phê duyệt</li>
      </ul>

      <div class="alert alert-warning mt-4">
        <span>⚠️ Thu hồi quyền database không ảnh hưởng requests đang PENDING.</span>
      </div>

      <h2>Approver Units</h2>
      <p><strong>Đường dẫn</strong>: Sidebar → "Approver Units"</p>
      <p>Đơn vị phê duyệt (VD: "DBA Team", "Security Team", "Data Owner"):</p>
      <ul>
        <li>Tạo / sửa / xóa units</li>
        <li>Gán users vào units (một user có thể thuộc nhiều units)</li>
        <li>Dùng trong Approval Matrix cho Data Queries</li>
      </ul>

      <h2>Approval Matrix</h2>
      <p><strong>Đường dẫn</strong>: Sidebar → "Approval Matrix"</p>
      <p>Cấu hình quy trình duyệt Data Queries theo database/schema/table:</p>

      <div class="overflow-x-auto">
        <table class="table table-sm table-zebra">
          <thead><tr><th>Database</th><th>Schema</th><th>Table</th><th>Approval Chain</th></tr></thead>
          <tbody>
            <tr><td>PROD-CRM</td><td>public</td><td>customers</td><td>DBA → Data Owner → Security</td></tr>
            <tr><td>PROD-CRM</td><td>public</td><td><em>(tất cả)</em></td><td>DBA → Data Owner</td></tr>
            <tr><td>PROD-CRM</td><td><em>(tất cả)</em></td><td><em>(tất cả)</em></td><td>DBA</td></tr>
            <tr><td>DEV-DB</td><td><em>(tất cả)</em></td><td><em>(tất cả)</em></td><td>DBA (1 level)</td></tr>
          </tbody>
        </table>
      </div>
      <p class="text-sm text-base-content/60 mt-2">
        Rule cụ thể hơn (database + schema + table) được ưu tiên hơn rule tổng quát.
      </p>
    </section>

    <!-- Section: Change Requests -->
    <section id="section-cr" class="mt-12">
      <h1>🔄 Change Requests — Tích hợp ITSM</h1>

      <p>SECS tích hợp với <strong>ManageEngine ServiceDesk Plus</strong> để liên kết SQL requests với Change Requests (CR).</p>

      <h2>Lợi ích</h2>
      <ul>
        <li><strong>Truy vết</strong>: SQL request nào thuộc change nào</li>
        <li><strong>Kiểm soát</strong>: Chỉ thực thi trong maintenance window</li>
        <li><strong>Tuân thủ</strong>: Audit trail đầy đủ ITSM ↔ DB changes</li>
      </ul>

      <h2>Tạo Change Request</h2>
      <p><strong>Đường dẫn</strong>: Sidebar → "Change Requests" → "Thêm CR"</p>
      <ol>
        <li>Dán URL từ ManageEngine</li>
        <li>Hệ thống tự động parse CR number</li>
        <li>Fetch metadata: title, status, maintenance window</li>
        <li>CR được lưu và sẵn sàng liên kết với SQL requests</li>
      </ol>

      <h2>Liên kết với SQL Request</h2>
      <p>Khi tạo SQL Request mới → dán URL ManageEngine vào trường "Change Request URL" → tự động liên kết.</p>

      <h2>Maintenance Window</h2>
      <div class="alert alert-info">
        <span>🕐 Khi request liên kết với CR có maintenance window, hệ thống kiểm tra thời gian thực thi có nằm trong window không. Ngoài window → cảnh báo hoặc từ chối.</span>
      </div>

      <h2>Đồng bộ & Xóa</h2>
      <ul>
        <li><strong>Sync</strong>: Cập nhật metadata mới nhất từ ManageEngine</li>
        <li><strong>Xóa</strong>: Chỉ Admin. SQL requests liên kết sẽ tự động gỡ liên kết</li>
      </ul>
    </section>

    <!-- Section: Notifications -->
    <section id="section-notifications" class="mt-12">
      <h1>🔔 Thông báo Realtime</h1>

      <p>SECS push thông báo realtime qua <strong>WebSocket</strong>. Tự động kết nối khi đăng nhập — không cần cấu hình.</p>

      <h2>Các loại thông báo</h2>
      <div class="overflow-x-auto">
        <table class="table table-sm table-zebra">
          <thead><tr><th>Loại</th><th>Nhận bởi</th><th>Mô tả</th></tr></thead>
          <tbody>
            <tr><td>New Request</td><td>Checker</td><td>Có request mới cần review</td></tr>
            <tr><td>Approved</td><td>Maker (owner)</td><td>Request được approve</td></tr>
            <tr><td>Rejected</td><td>Maker (owner)</td><td>Request bị reject</td></tr>
            <tr><td>Executed</td><td>Maker (owner)</td><td>SQL thực thi thành công</td></tr>
            <tr><td>Failed</td><td>Maker (owner)</td><td>SQL thực thi lỗi</td></tr>
            <tr><td>Partial Approval</td><td>Maker (owner)</td><td>Được approve 1 level (chưa đủ)</td></tr>
          </tbody>
        </table>
      </div>

      <h2>Filtering theo Role</h2>
      <ul>
        <li><strong>Admin</strong>: Nhận tất cả thông báo</li>
        <li><strong>Maker</strong>: Chỉ thông báo liên quan request của mình</li>
        <li><strong>Checker</strong>: Request mới + request đã từng review</li>
      </ul>

      <p class="text-sm text-base-content/60 mt-2">
        Thông báo hiển thị dạng Toast (popup góc phải), tự ẩn sau vài giây. Mất kết nối → tự reconnect.
      </p>
    </section>

    <!-- Section: AI -->
    <section id="section-ai" class="mt-12">
      <h1>🤖 Phân tích SQL bằng AI</h1>

      <p>SECS tích hợp AI (LLM) để phân tích SQL, giúp phát hiện sớm vấn đề performance, security, và logic.</p>

      <h2>Cách sử dụng</h2>
      <ol>
        <li>Mở chi tiết SQL Request (DRAFT hoặc PENDING)</li>
        <li>Nhấn nút <kbd class="kbd kbd-sm">🤖 AI Analysis</kbd></li>
        <li>Chờ vài giây — hệ thống phân tích</li>
        <li>Xem kết quả chi tiết</li>
      </ol>

      <h2>AI phân tích những gì?</h2>
      <div class="grid grid-cols-1 md:grid-cols-2 gap-4 not-prose mt-4">
        <div class="card bg-base-200 p-4">
          <h4 class="font-bold mb-2">🎯 Risk Score (1-10)</h4>
          <ul class="text-sm space-y-1">
            <li><span class="badge badge-success badge-sm">1-3</span> Thấp — Safe</li>
            <li><span class="badge badge-warning badge-sm">4-6</span> Trung bình — Cần review kỹ</li>
            <li><span class="badge badge-error badge-sm">7-10</span> Cao — Cẩn thận</li>
          </ul>
        </div>
        <div class="card bg-base-200 p-4">
          <h4 class="font-bold mb-2">⚡ Performance Issues</h4>
          <ul class="text-sm space-y-1">
            <li>• Full table scan (thiếu index)</li>
            <li>• Query quá phức tạp</li>
            <li>• JOIN nhiều bảng lớn</li>
          </ul>
        </div>
        <div class="card bg-base-200 p-4">
          <h4 class="font-bold mb-2">🔒 Security Issues</h4>
          <ul class="text-sm space-y-1">
            <li>• Thao tác dữ liệu nhạy cảm</li>
            <li>• Missing WHERE clause</li>
            <li>• Privilege escalation risk</li>
          </ul>
        </div>
        <div class="card bg-base-200 p-4">
          <h4 class="font-bold mb-2">💡 Suggestions</h4>
          <ul class="text-sm space-y-1">
            <li>• Thêm WHERE clause</li>
            <li>• Sử dụng transaction</li>
            <li>• Backup trước khi thực thi</li>
          </ul>
        </div>
      </div>

      <h2>Ai có thể sử dụng?</h2>
      <ul>
        <li><strong>Maker</strong>: Phân tích request của mình (DRAFT hoặc PENDING)</li>
        <li><strong>Checker</strong>: Phân tích request đang PENDING (đang review)</li>
      </ul>

      <h2>Giới hạn</h2>
      <ul>
        <li>Rate limit: giới hạn số lần phân tích mỗi user</li>
        <li>Kết quả được cache — cùng SQL không gọi AI lần 2</li>
        <li>Có thể bị tắt bởi Admin qua cấu hình server</li>
      </ul>
    </section>

    <!-- Section: Rules -->
    <section id="section-rules" class="mt-12">
      <h1>📜 Quy tắc nghiệp vụ quan trọng</h1>

      <div class="space-y-4 not-prose">
        <div class="collapse collapse-arrow bg-base-200">
          <input type="radio" name="rules-accordion" checked />
          <div class="collapse-title font-bold">
            👁️ Nguyên tắc 4-Eyes (Four-Eyes Principle)
          </div>
          <div class="collapse-content text-sm">
            <p>Người tạo request <strong>KHÔNG THỂ</strong> tự approve request đó.</p>
            <ul class="list-disc ml-4 mt-2">
              <li>Dù user có cả role Maker và Checker</li>
              <li>Hệ thống kiểm tra: created_by ≠ approved_by</li>
              <li>Vi phạm → HTTP 403 (FourEyesViolation)</li>
            </ul>
          </div>
        </div>

        <div class="collapse collapse-arrow bg-base-200">
          <input type="radio" name="rules-accordion" />
          <div class="collapse-title font-bold">
            🔒 Immutable Request (Không sửa sau submit)
          </div>
          <div class="collapse-content text-sm">
            <p>Sau khi submit (PENDING), nội dung SQL <strong>không thể chỉnh sửa</strong>.</p>
            <ul class="list-disc ml-4 mt-2">
              <li>Chỉ DRAFT mới cho phép edit</li>
              <li>Muốn thay đổi: Cancel → Clone → Sửa → Submit lại</li>
            </ul>
          </div>
        </div>

        <div class="collapse collapse-arrow bg-base-200">
          <input type="radio" name="rules-accordion" />
          <div class="collapse-title font-bold">
            📝 Audit Trail bắt buộc
          </div>
          <div class="collapse-content text-sm">
            <p>Mọi hành động trên request đều được ghi log:</p>
            <ul class="list-disc ml-4 mt-2">
              <li>Hành động: CREATED, SUBMITTED, APPROVED, REJECTED, EXECUTED, FAILED, CANCELLED</li>
              <li>Người thực hiện + thời gian + địa chỉ IP</li>
              <li>Chi tiết: lý do reject, error message, rows affected</li>
            </ul>
          </div>
        </div>

        <div class="collapse collapse-arrow bg-base-200">
          <input type="radio" name="rules-accordion" />
          <div class="collapse-title font-bold">
            ⚠️ DDL Warning
          </div>
          <div class="collapse-content text-sm">
            <p>Câu lệnh DDL (DROP, TRUNCATE, ALTER, CREATE) được flag đặc biệt:</p>
            <ul class="list-disc ml-4 mt-2">
              <li>Checker thấy cảnh báo rõ ràng khi review</li>
              <li>DDL thường cần nhiều approval levels hơn</li>
              <li>Maker nên ghi rõ lý do trong description</li>
            </ul>
          </div>
        </div>

        <div class="collapse collapse-arrow bg-base-200">
          <input type="radio" name="rules-accordion" />
          <div class="collapse-title font-bold">
            ⚛️ Thực thi nguyên tử (Atomic Execution)
          </div>
          <div class="collapse-content text-sm">
            <p>Mọi SQL execution được wrap trong transaction:</p>
            <ul class="list-disc ml-4 mt-2">
              <li>Nhiều statements → tất cả thành công hoặc tất cả rollback</li>
              <li>Timeout → rollback + ghi log FAILED</li>
              <li>Kết nối TLS bắt buộc</li>
            </ul>
          </div>
        </div>

        <div class="collapse collapse-arrow bg-base-200">
          <input type="radio" name="rules-accordion" />
          <div class="collapse-title font-bold">
            🔑 Phân quyền Database
          </div>
          <div class="collapse-content text-sm">
            <p>Không có quyền global — phải cấp explicit:</p>
            <ul class="list-disc ml-4 mt-2">
              <li>Admin cấp quyền user → database</li>
              <li>Maker chỉ thấy databases mình có quyền</li>
              <li>Thu hồi quyền → không ảnh hưởng requests đang PENDING</li>
            </ul>
          </div>
        </div>
      </div>
    </section>

    <!-- Section: FAQ -->
    <section id="section-faq" class="mt-12 mb-12">
      <h1>❓ FAQ — Câu hỏi thường gặp</h1>

      <div class="space-y-3 not-prose">
        <div class="collapse collapse-arrow bg-base-200">
          <input type="radio" name="faq-accordion" checked />
          <div class="collapse-title font-medium">
            Tôi không thấy database nào khi tạo request?
          </div>
          <div class="collapse-content text-sm">
            <p>Admin chưa cấp quyền cho bạn trên database nào. Liên hệ Admin để được cấp quyền (Admin → Users → [Tên bạn] → Database Permissions → Thêm quyền).</p>
          </div>
        </div>

        <div class="collapse collapse-arrow bg-base-200">
          <input type="radio" name="faq-accordion" />
          <div class="collapse-title font-medium">
            Request bị reject, làm sao sửa lại?
          </div>
          <div class="collapse-content text-sm">
            <p>Request đã submit không thể sửa. Nhấn <strong>"Clone"</strong> để tạo bản sao DRAFT, chỉnh sửa SQL theo feedback, rồi submit lại.</p>
          </div>
        </div>

        <div class="collapse collapse-arrow bg-base-200">
          <input type="radio" name="faq-accordion" />
          <div class="collapse-title font-medium">
            Tôi có cả role Maker và Checker, sao không approve được request của mình?
          </div>
          <div class="collapse-content text-sm">
            <p>Đây là <strong>4-eyes principle</strong>. Mọi SQL request phải được người khác review. Nhờ một Checker khác approve.</p>
          </div>
        </div>

        <div class="collapse collapse-arrow bg-base-200">
          <input type="radio" name="faq-accordion" />
          <div class="collapse-title font-medium">
            SQL thực thi bị timeout?
          </div>
          <div class="collapse-content text-sm">
            <p>Timeout cấu hình theo từng database (1-300 giây). Giải pháp:</p>
            <ul class="list-disc ml-4 mt-1">
              <li>Dùng AI Analysis kiểm tra SQL có phức tạp không</li>
              <li>Chia nhỏ thành nhiều request</li>
              <li>Liên hệ Admin tăng timeout</li>
            </ul>
          </div>
        </div>

        <div class="collapse collapse-arrow bg-base-200">
          <input type="radio" name="faq-accordion" />
          <div class="collapse-title font-medium">
            Checker approve rồi nhưng SQL chưa thực thi?
          </div>
          <div class="collapse-content text-sm">
            <p>Request yêu cầu <strong>multi-level approval</strong>. Kiểm tra tiến trình (ví dụ: "1/2 approvals"). Cần thêm Checker khác approve.</p>
          </div>
        </div>

        <div class="collapse collapse-arrow bg-base-200">
          <input type="radio" name="faq-accordion" />
          <div class="collapse-title font-medium">
            Tôi là Checker nhưng không thấy request trong danh sách chờ duyệt?
          </div>
          <div class="collapse-content text-sm">
            <p>Có thể vì:</p>
            <ul class="list-disc ml-4 mt-1">
              <li>Request do chính bạn tạo (4-eyes — tự động ẩn)</li>
              <li>Request đã bị Cancel hoặc đã đủ approval</li>
              <li>Bạn chưa có role Checker</li>
            </ul>
          </div>
        </div>

        <div class="collapse collapse-arrow bg-base-200">
          <input type="radio" name="faq-accordion" />
          <div class="collapse-title font-medium">
            Data Query approved nhưng không thể execute?
          </div>
          <div class="collapse-content text-sm">
            <p>Kiểm tra:</p>
            <ul class="list-disc ml-4 mt-1">
              <li>Query có hết hạn (expires_at) chưa?</li>
              <li>Bạn có phải người tạo hoặc assigned user không?</li>
              <li>Database đích có đang active không?</li>
            </ul>
          </div>
        </div>

        <div class="collapse collapse-arrow bg-base-200">
          <input type="radio" name="faq-accordion" />
          <div class="collapse-title font-medium">
            WebSocket thông báo không hoạt động?
          </div>
          <div class="collapse-content text-sm">
            <p>Thử:</p>
            <ul class="list-disc ml-4 mt-1">
              <li>Refresh trang (F5) — hệ thống tự reconnect</li>
              <li>Kiểm tra browser không block WebSocket (proxy, firewall)</li>
              <li>Token hết hạn → đăng nhập lại</li>
            </ul>
          </div>
        </div>

        <div class="collapse collapse-arrow bg-base-200">
          <input type="radio" name="faq-accordion" />
          <div class="collapse-title font-medium">
            Schema browser không hiển thị tables?
          </div>
          <div class="collapse-content text-sm">
            <p>Kiểm tra:</p>
            <ul class="list-disc ml-4 mt-1">
              <li>Bạn có quyền trên database đó không?</li>
              <li>Database connection đang active?</li>
              <li>Network SECS → database đích thông?</li>
              <li>Schema name đúng? (PostgreSQL: public, MySQL: tên database)</li>
            </ul>
          </div>
        </div>

        <div class="collapse collapse-arrow bg-base-200">
          <input type="radio" name="faq-accordion" />
          <div class="collapse-title font-medium">
            ManageEngine CR không sync được metadata?
          </div>
          <div class="collapse-content text-sm">
            <p>Kiểm tra:</p>
            <ul class="list-disc ml-4 mt-1">
              <li>ManageEngine integration có được bật không?</li>
              <li>URL đúng format?</li>
              <li>Network SECS server → ManageEngine thông?</li>
              <li>API credentials ManageEngine còn hợp lệ?</li>
            </ul>
          </div>
        </div>
      </div>

      <div class="divider mt-8"></div>
      <p class="text-center text-sm text-base-content/50">
        SECS — SQL Execution Control System v1.0<br />
        Liên hệ Admin nếu cần hỗ trợ thêm.
      </p>
    </section>
  </main>
</div>

<style>
  /* Section separator & heading styling */
  main :global(section[id^="section-"]:not(:first-child)) {
    margin-top: 2.5rem;
    padding-top: 2rem;
    border-top: 2px solid rgba(128, 128, 128, 0.15);
  }

  /* Section main heading (h1) styling */
  main :global(section[id^="section-"] > h1:first-of-type) {
    font-size: 1.5rem;
    font-weight: 800;
    padding-bottom: 0.75rem;
    margin-bottom: 1.25rem;
    border-bottom: 1px solid rgba(128, 128, 128, 0.12);
  }
</style>
