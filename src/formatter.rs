use ironworks::sestring::{
    Error as SeStringError, SeString,
    format::{Color, ColorUsage, Input, Style, Write, format},
};

/// Format an sestring with HTML tags
pub fn format_string(string: &SeString, input: &Input) -> String {
    let mut writer = TagWriter::default();
    let _ = format(string.as_ref(), &input, &mut writer);

    return writer.buffer;
}

#[derive(Debug, Default)]
struct TagWriter {
    buffer: String,
}

impl Write for TagWriter {
    // Potentially do some mass replacements here
    fn write_str(&mut self, str: &str) -> Result<(), SeStringError> {
        self.buffer.push_str(&str);

        Ok(())
    }

    // Format styled text with HTML tags
    fn set_style(&mut self, style: Style, enabled: bool) -> Result<(), SeStringError> {
        let tag = match style {
            Style::Bold => "b",
            Style::Italic => "i",
            _ => return Ok(()),
        };

        let close = match enabled {
            true => "",
            false => "/",
        };

        self.buffer.push_str(&format!("<{}{}>", close, tag));

        Ok(())
    }

    // Just replace all of the color shenanigans with <b> tags
    fn push_color(&mut self, usage: ColorUsage, _color: Color) -> Result<(), SeStringError> {
        if usage == ColorUsage::Foreground {
            self.buffer.push_str("<b>");
        }

        Ok(())
    }

    fn pop_color(&mut self, usage: ColorUsage) -> Result<(), SeStringError> {
        if usage != ColorUsage::Foreground {
            self.buffer.push_str("</b>");
        }

        Ok(())
    }
}
