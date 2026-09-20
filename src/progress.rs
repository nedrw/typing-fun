//! 成绩档案的纯计算：把一串速度值折成迷你趋势线的坐标。

/// 把速度序列折成 SVG polyline 的坐标串（viewBox 由调用方给定）。
///
/// 少于两个点画不出线，返回 `None`。
pub fn spark_points(values: &[f64], width: f64, height: f64) -> Option<String> {
    if values.len() < 2 {
        return None;
    }
    let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    // 全相等时留 1 的跨度，避免除零，也免得微小波动被放大成锯齿
    let span = (max - min).max(1.0);
    let last = (values.len() - 1) as f64;
    let points: Vec<String> = values
        .iter()
        .enumerate()
        .map(|(i, value)| {
            let x = i as f64 / last * width;
            let y = height - (value - min) / span * height;
            format!("{x:.1},{y:.1}")
        })
        .collect();
    Some(points.join(" "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fewer_than_two_points_draws_nothing() {
        assert_eq!(spark_points(&[], 100.0, 30.0), None);
        assert_eq!(spark_points(&[42.0], 100.0, 30.0), None);
    }

    #[test]
    fn points_span_the_whole_width() {
        let points = spark_points(&[10.0, 20.0, 30.0], 100.0, 30.0).unwrap();
        let xs: Vec<f64> = points
            .split(' ')
            .map(|p| p.split(',').next().unwrap().parse().unwrap())
            .collect();
        assert_eq!(xs.first().copied(), Some(0.0));
        assert_eq!(xs.last().copied(), Some(100.0));
    }

    #[test]
    fn faster_values_sit_higher() {
        let points = spark_points(&[10.0, 20.0], 100.0, 30.0).unwrap();
        let ys: Vec<f64> = points
            .split(' ')
            .map(|p| p.split(',').nth(1).unwrap().parse().unwrap())
            .collect();
        assert!(ys[1] < ys[0], "更快的成绩 y 更小（更靠上）");
    }

    #[test]
    fn equal_values_stay_inside_the_box() {
        let points = spark_points(&[30.0, 30.0, 30.0], 100.0, 30.0).unwrap();
        for point in points.split(' ') {
            let y: f64 = point.split(',').nth(1).unwrap().parse().unwrap();
            assert!(y.is_finite() && (0.0..=30.0).contains(&y), "y={y}");
        }
    }
}
