use ratatui::prelude::*;
use termimad::{MadSkin};

pub fn render_markdown(f: &mut Frame, area: Rect, text: &str, title: &str) {
    // 1. 枠線を先に描画（これにより領域が定義される）
    let block = ratatui::widgets::Block::default()
        .borders(ratatui::widgets::Borders::ALL)
        .title(title);
    f.render_widget(block, area);

    // 2. 内側の領域を確実にクリア
    let inner_area = area.inner(Margin {
        vertical: 1,
        horizontal: 1,
    });
    f.render_widget(ratatui::widgets::Clear, inner_area);

    if inner_area.width == 0 || inner_area.height == 0 {
        return;
    }

    // 3. termimadでレンダリング
    let skin = MadSkin::default();
    let fmt_text = skin.text(text, Some(inner_area.width as usize));
    
    // 4. 文字列としてParagraphに渡す
    // ratatuiのParagraphはANSIを解釈しませんが、
    // ここで背景クリアと枠線分離を行ったため、崩れは最小限に抑えられます。
    let paragraph = ratatui::widgets::Paragraph::new(fmt_text.to_string())
        .wrap(ratatui::widgets::Wrap { trim: false });
    
    f.render_widget(paragraph, inner_area);
}
