//! 常驻组件按实际占位排布，并在开关变化时连续补位。
use crate::render::anim::{smoothstep, Spring};

pub(crate) const CLOCK: usize = 0;
pub(crate) const BATTERY: usize = 1;
pub(crate) const QUOTA: usize = 2;
/// 可独立排列的天气徽标。
pub(crate) const WEATHER: usize = 5;
pub(crate) fn quota_slot(position:usize)->usize {
    if position<3 {QUOTA+position} else {WIDTHS.len()+position-3}
}
pub(crate) const MUSIC: usize = 6;
pub(crate) const FOCUS: usize = 7;
pub(crate) const BELL: usize = 8;
pub(crate) const PET: usize = 9;
/// AI 会话状态灯：运行 / 等待 / 可能中断。
pub(crate) const AGENT: usize = 10;
/// 事件容器：固定在铃铛左侧，只在有事件时占位。
pub(crate) const TRAY: usize = 11;
/// 页面徽标：有未读 / 到期 / 提醒或刚使用时短暂占位，点击进入对应功能页。
pub(crate) const MAIL: usize = 12;
pub(crate) const HEALTH: usize = 13;
pub(crate) const TODO: usize = 14;
pub(crate) const SHELF: usize = 15;
pub(crate) const CALENDAR: usize = 16;
pub(crate) const DND: usize = 17;
/// 天气徽标的基础槽宽；温度文字更长时由 `widen` 补足。
pub(crate) const WEATHER_WIDTH: f32 = 38.0;
/// 两端内边距与组件间距。
pub(crate) const EDGE_PAD: f32 = 10.0;
pub(crate) const GAP: f32 = 8.0;
/// 展开态空间不足时先压缩间距，压缩到这个值仍放不下才整体等比缩小。
pub(crate) const MIN_GAP: f32 = 4.0;
/// 状态行简单图标的统一视觉尺寸；外框 20 点，图形留出约 3 点内边距。
/// 电池环、音乐柱、铃铛按自身轮廓缩放到它。
pub(crate) const GLYPH: f32 = 14.0;
pub(crate) const BATTERY_SCALE: f32 = GLYPH / 19.0;
pub(crate) const BELL_SCALE: f32 = GLYPH / 9.8;
/// 音柱外框的宽高比与图标一致，宽度取 GLYPH 时高度约为 0.875 倍。
pub(crate) const MUSIC_H: f32 = GLYPH * 0.875;
/// 齿轮轮廓直径约 15.2 点，月亮外圆直径约 1.24 倍 size；都归一到 GLYPH。
pub(crate) const GEAR_SCALE: f32 = GLYPH / 15.2;
pub(crate) const MOON_SIZE: f32 = GLYPH / 1.24;
/// 展开态尾部控件（勿扰、设置）占用的宽度：两个槽加两个间距。
pub(crate) const TAIL_SLOT: f32 = 20.0;
pub(crate) const TAIL_W: f32 = TAIL_SLOT * 2.0 + GAP * 2.0;
/// AI 状态灯是 9 点圆点，闲置圆环外径约 10 点，等待光晕 16 点；槽宽取 14 点，避免圆点两侧留下过大空白。
pub(crate) const AGENT_SLOT: f32 = 14.0;
/// 简单图标的统一外框 20 点，内部图形约 14 点；徽标含数量时用 30，宠物也用 20 点槽宽，带数据的组件保持各自宽度。
const WIDTHS: [f32; 18] = [44.0,20.0,27.0,27.0,27.0,WEATHER_WIDTH,20.0,crate::app::focus_bar::WIDTH,20.0,20.0,AGENT_SLOT,0.0,0.0,0.0,0.0,0.0,0.0,0.0];

/// 固定槽位的基础宽度；额度溢出槽不在此列。
pub(crate) fn slot_width(slot:usize)->f32 {WIDTHS.get(slot).copied().unwrap_or(0.0)}

pub(crate) const DEFAULT_ORDER: [usize;8]=[BATTERY,PET,CLOCK,WEATHER,QUOTA,MUSIC,FOCUS,AGENT];
pub(crate) fn valid_order(order:&[usize])->bool {
    order.len()==DEFAULT_ORDER.len() && DEFAULT_ORDER.iter().all(|slot|order.iter().filter(|value|*value==slot).count()==1)
}
/// 事件容器成员的默认顺序；预览里可拖动调整。
pub(crate) const TRAY_ORDER: [usize;10]=[WEATHER,QUOTA,FOCUS,MAIL,HEALTH,TODO,SHELF,CALENDAR,MUSIC,BATTERY];
pub(crate) fn valid_tray_order(order:&[usize])->bool {
    order.len()==TRAY_ORDER.len() && TRAY_ORDER.iter().all(|slot|order.iter().filter(|value|*value==slot).count()==1)
}
/// 设置页「岛上显示」两组的固定展示顺序；岛上排布仍由预览和 `resident_order` / `tray_order` 决定。
pub(crate) const GROUP_ORDER:[usize;14]=[AGENT,PET,BATTERY,CLOCK,DND,MUSIC,FOCUS,QUOTA,CALENDAR,WEATHER,MAIL,HEALTH,TODO,SHELF];
pub(crate) fn animations_enabled()->bool {
    #[cfg(windows)] {
        static ENABLED:std::sync::OnceLock<bool>=std::sync::OnceLock::new();
        *ENABLED.get_or_init(||{
            use windows::Win32::{Foundation::BOOL,UI::WindowsAndMessaging::*};
            let mut enabled=BOOL(1);
            unsafe {let _=SystemParametersInfoW(SPI_GETCLIENTAREAANIMATION,0,Some((&mut enabled as *mut BOOL).cast()),SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0));}
            enabled.as_bool()
        })
    }
    #[cfg(not(windows))] {true}
}

#[derive(Clone, Debug)]
pub(crate) struct Layout {
    pub width: f32,
    pub x: Vec<Option<f32>>,
    pub widths:Vec<f32>,
    quotas:usize,
    gap:f32,
}

impl Layout {
    pub fn new(battery:bool,quotas:usize,music:bool,focus:bool,bell:bool,pet:bool,weather:bool,agent:bool)->Self {
        Self::new_with_gap(GAP,battery,quotas,music,focus,bell,pet,weather,agent)
    }
    /// 自定义组件间距；岛体用 `GAP`，预览编辑器用更紧凑的统一间距。
    pub fn new_with_gap(gap:f32,battery:bool,quotas:usize,music:bool,focus:bool,bell:bool,pet:bool,weather:bool,agent:bool)->Self {
        fn place(cursor:&mut f32,width:f32,gap:f32)->f32 {let x=*cursor+width/2.0;*cursor+=width+gap;x}
        let mut widths=WIDTHS.to_vec(); widths.resize(WIDTHS.len()+quotas.saturating_sub(3),27.0);
        let mut x=vec![None;widths.len()];let mut cursor=EDGE_PAD;
        let count=quotas;
        let order=if battery {[BATTERY,CLOCK,QUOTA]}else{[QUOTA,CLOCK,BATTERY]};
        for slot in order {
            match slot {
                BATTERY if battery=>x[slot]=Some(place(&mut cursor,WIDTHS[slot],gap)),
                CLOCK=>{
                    if pet {x[PET]=Some(place(&mut cursor,WIDTHS[PET],gap));}
                    x[slot]=Some(place(&mut cursor,WIDTHS[slot],gap));
                    if weather {x[WEATHER]=Some(place(&mut cursor,WIDTHS[WEATHER],gap));}
                },
                QUOTA if count>0=>{
                    let width=count as f32*27.0+(count-1) as f32*7.0;
                    let left=place(&mut cursor,width,gap)-width/2.0;
                    for i in 0..count {x[quota_slot(i)]=Some(left+13.5+i as f32*34.0);}
                },
                _=>{},
            }
        }
        for (slot,on) in [(MUSIC,music),(FOCUS,focus),(BELL,bell),(AGENT,agent)] {
            if on {x[slot]=Some(place(&mut cursor,WIDTHS[slot],gap));}
        }
        let width=cursor-gap+EDGE_PAD;
        for value in x.iter_mut().flatten() {*value-=width/2.0;}
        Self {width:width.max(96.0),x,widths,quotas,gap}
    }
    pub fn quota_slots(&self)->impl Iterator<Item=usize> { (0..self.quotas).map(quota_slot) }
    /// 把某个槽加宽 extra：其余槽对称让位并记入总宽。点数文字、时段时钟、天气温度共用。
    pub fn widen(&mut self,slot:usize,extra:f32){
        if extra<=0.0 {return;}
        let Some(center)=self.x.get(slot).copied().flatten() else {return;};
        for (i,x) in self.x.iter_mut().enumerate(){if i!=slot{if let Some(x)=x{*x+=if *x<center{-extra/2.0}else{extra/2.0};}}}
        self.widths[slot]+=extra;self.width+=extra;
    }
    pub fn balance_tail(&mut self,position:usize,extra:f32){self.widen(quota_slot(position),extra);}
    pub fn widen_clock(&mut self, extra:f32) {self.widen(CLOCK,extra);}
    pub fn hide_clock(&mut self) {
        let Some(center)=self.x[CLOCK].take() else{return;};
        let removed=self.widths[CLOCK]+self.gap;
        self.width=(self.width-removed).max(96.0);
        for x in self.x.iter_mut().flatten() {*x+=if *x<center {removed/2.0}else{-removed/2.0};}
    }
    /// 在铃铛左侧插入事件容器；没有铃铛时容器就是最右组件。调用前须已 `reorder`（铃铛在最右）。
    pub fn add_tray(&mut self,width:f32) {
        if width<=0.0 {return;}
        let extra=width+self.gap;
        self.widths[TRAY]=width;
        for (i,x) in self.x.iter_mut().enumerate() {
            if let Some(x)=x {*x+=if i==BELL {extra/2.0}else{-extra/2.0};}
        }
        let right=match self.x[BELL] {
            Some(bell)=>bell-self.widths[BELL]/2.0-self.gap,
            None=>{
                let edge=self.x.iter().zip(&self.widths).filter_map(|(x,w)|x.map(|x|x+w/2.0)).fold(f32::NEG_INFINITY,f32::max);
                if edge.is_finite() {edge+self.gap+width}else{width/2.0}
            }
        };
        self.x[TRAY]=Some(right-width/2.0);
        let left=self.x.iter().zip(&self.widths).filter_map(|(x,w)|x.map(|x|x-w/2.0)).fold(f32::INFINITY,f32::min);
        let rightmost=self.x.iter().zip(&self.widths).filter_map(|(x,w)|x.map(|x|x+w/2.0)).fold(f32::NEG_INFINITY,f32::max);
        self.width=((rightmost-left)+2.0*EDGE_PAD).max(96.0);
    }
    /// 在铃铛左侧插入页面徽标；`items` 从左到右排列。调用前须已 `reorder`。
    pub fn add_extras(&mut self,items:&[(usize,f32)]) {
        let items:Vec<_>=items.iter().copied().filter(|(_,width)|*width>0.0).collect();
        if items.is_empty() {return;}
        let content: f32=items.iter().map(|(_,width)|*width).sum::<f32>()+self.gap*(items.len()-1) as f32;
        let extra=content+self.gap;
        for (slot,width) in &items {self.widths[*slot]=*width;}
        for (i,x) in self.x.iter_mut().enumerate() {
            if let Some(x)=x {*x+=if i==BELL {extra/2.0}else{-extra/2.0};}
        }
        let mut right=match self.x[BELL] {
            Some(bell)=>bell-self.widths[BELL]/2.0-self.gap,
            None=>{
                let edge=self.x.iter().zip(&self.widths).filter_map(|(x,w)|x.map(|x|x+w/2.0)).fold(f32::NEG_INFINITY,f32::max);
                if edge.is_finite() {edge+self.gap+content}else{content/2.0}
            }
        };
        if !right.is_finite() {right=content/2.0;}
        for (slot,width) in items.iter().rev() {
            self.x[*slot]=Some(right-width/2.0);
            right-=width+self.gap;
        }
        let left=self.x.iter().zip(&self.widths).filter_map(|(x,w)|x.map(|x|x-w/2.0)).fold(f32::INFINITY,f32::min);
        let rightmost=self.x.iter().zip(&self.widths).filter_map(|(x,w)|x.map(|x|x+w/2.0)).fold(f32::NEG_INFINITY,f32::max);
        self.width=((rightmost-left)+2.0*EDGE_PAD).max(96.0);
    }
    /// 按实际内容重新居中并返回内容宽度；用于只有额度组的迷你布局。
    pub fn center_content(&mut self)->f32 {
        let(mut left,mut right)=(f32::INFINITY,f32::NEG_INFINITY);
        for (i,x) in self.x.iter().enumerate() {
            if let Some(x)=x {left=left.min(x-self.widths[i]/2.0);right=right.max(x+self.widths[i]/2.0);}
        }
        if !left.is_finite() {return 0.0;}
        let center=(left+right)/2.0;
        for x in self.x.iter_mut().flatten() {*x-=center;}
        self.width=right-left;
        right-left
    }
    pub fn reorder(&mut self,order:&[usize]) {
        if !valid_order(order){return;}
        let original=self.x.clone();let mut cursor=-self.width/2.0+EDGE_PAD;
        for slot in order.iter().copied().chain(std::iter::once(BELL)) {
            let slots:Vec<usize>=if slot==QUOTA{self.quota_slots().collect()}else{vec![slot]};
            let left=slots.iter().filter_map(|&i|original[i].map(|x|x-self.widths[i]/2.0)).fold(f32::INFINITY,f32::min);
            let right=slots.iter().filter_map(|&i|original[i].map(|x|x+self.widths[i]/2.0)).fold(f32::NEG_INFINITY,f32::max);
            if !left.is_finite(){continue;}
            for i in slots {if let Some(x)=original[i]{self.x[i]=Some(cursor+x-left);}}
            cursor+=right-left+self.gap;
        }
    }
    /// 圆环换边经过时钟时，暂时淡出时钟，避免移动中相互遮挡。
    pub fn clearance(&self,slot:usize)->f32 {
        let Some(x)=self.x[slot] else{return 0.0;};
        let distance=self.quota_slots().filter_map(|i|self.x[i].map(|other|(other-x).abs()-(self.widths[i]-27.0)/2.0)).fold(f32::INFINITY,f32::min);
        let quota=if slot==CLOCK {smoothstep(30.0,40.0,distance)}else{smoothstep(18.0,28.0,distance)};
        quota*self.pet_clearance(slot)
    }
    pub fn pet_clearance(&self,slot:usize)->f32 {
        match (self.x[slot],self.x[PET]) {
            (Some(x),Some(pet))=>smoothstep(self.widths[slot]/2.0+10.0,self.widths[slot]/2.0+18.0,(x-pet).abs()),
            _=>1.0,
        }
    }
}

pub(crate) struct Motion {
    pub target: Layout,
    positions: Vec<Spring>,
    initialized: bool,
}
impl Default for Motion {
    fn default()->Self {Self {target:Layout::new(false,0,false,false,false,false,false,false),positions:vec![Spring::new(0.0,360.0,0.76);WIDTHS.len()],initialized:false}}
}
impl Motion {
    pub fn step(&mut self,target:Layout,dt:f32,animate:bool) {
        self.positions.resize(target.x.len(),Spring::new(0.0,360.0,0.76));
        for (i,x) in target.x.iter().enumerate() {
            if let Some(x)=x {
                let spring=&mut self.positions[i];spring.target=*x;
                if !self.initialized || self.target.x.get(i).copied().flatten().is_none() || !animate {spring.snap(*x);}else{spring.step(dt);}
            }
        }
        self.target=target;self.initialized=true;
    }
    pub fn current(&self)->Layout {
        let mut layout=self.target.clone();
        for (i,x) in layout.x.iter_mut().enumerate() {if x.is_some(){*x=Some(self.positions[i].pos);}}
        layout
    }
    pub fn position(&self,slot:usize)->f32 {self.positions[slot].pos}
    pub fn settled(&self)->bool {self.target.x.iter().enumerate().all(|(i,x)|x.is_none()||self.positions[i].settled(100.0))}
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tail_glyphs_match_the_status_row_glyph_box(){
        assert!((GEAR_SCALE*15.2-GLYPH).abs()<0.01,"齿轮外径应归一到统一图标尺寸");
        assert!((MOON_SIZE*1.24-GLYPH).abs()<0.01,"月亮外圆直径应归一到统一图标尺寸");
    }
    #[test]
    fn hidden_clock_closes_its_gap_in_custom_orders(){
        for reverse in [false,true] {for count in [0,5] {
            let mut order=DEFAULT_ORDER.to_vec();if reverse {order.reverse();}
            let mut layout=Layout::new(true,count,true,true,false,true,true,false);
            layout.reorder(&order);layout.widen_clock(64.0);
            let width=layout.width;let removed=layout.widths[CLOCK]+GAP;
            layout.hide_clock();assert!(layout.x[CLOCK].is_none());
            assert_eq!(layout.width,(width-removed).max(96.0));
            let mut boxes:Vec<_>=layout.x.iter().enumerate().filter_map(|(i,x)|x.map(|x|(x-layout.widths[i]/2.0,x+layout.widths[i]/2.0))).collect();
            boxes.sort_by(|a,b|a.0.total_cmp(&b.0));
            assert!(boxes.windows(2).all(|p|p[1].0-p[0].1>=6.9),"关闭时间后相邻组件不应重叠");
            let after=layout.x.clone();layout.hide_clock();assert_eq!(layout.x,after);
        }}
        let mut empty=Layout::new(false,0,false,false,false,false,false,false);
        empty.hide_clock();assert_eq!(empty.width,96.0);assert!(empty.x.iter().all(Option::is_none));
    }
    #[test]
    fn weather_can_be_reordered_independently_of_the_clock(){
        let order=[WEATHER,FOCUS,MUSIC,QUOTA,CLOCK,PET,BATTERY,AGENT];
        assert!(valid_order(&order),"天气应有独立的排列位置");
        for quotas in 0..=5 {for enabled in [false,true] {
            let mut layout=Layout::new(true,quotas,true,true,false,true,enabled,false);
            layout.reorder(&order);layout.widen(WEATHER,18.0);layout.widen_clock(40.0);
            let positions:Vec<_>=order.iter().filter_map(|slot|layout.x[*slot]).collect();
            assert!(positions.windows(2).all(|pair|pair[0]<pair[1]));
            let boxes:Vec<_>=order.iter().flat_map(|&slot|if slot==QUOTA{layout.quota_slots().collect()}else{vec![slot]}).filter_map(|slot|layout.x[slot].map(|x|(x-layout.widths[slot]/2.0,x+layout.widths[slot]/2.0))).collect();
            assert!(boxes.windows(2).all(|pair|pair[1].0-pair[0].1>=6.9),"排序和加宽后组件不能重叠");
        }}
    }
    #[test] fn deepseek_balance_follows_more_and_never_overlaps(){
        for order in [DEFAULT_ORDER,[FOCUS,MUSIC,QUOTA,CLOCK,WEATHER,PET,BATTERY,AGENT]] {for quotas in 1..=6 {for battery in [false,true]{
            let mut layout=Layout::new(battery,quotas,true,true,false,true,false,false);layout.reorder(&order);layout.balance_tail(quotas-1,76.0);layout.widen_clock(40.0);
            let mut boxes:Vec<_>=layout.x.iter().enumerate().filter_map(|(i,x)|x.map(|x|{let width=layout.widths[i];(x-width/2.0,x+width/2.0)})).collect();boxes.sort_by(|a,b|a.0.total_cmp(&b.0));
            assert!(boxes.windows(2).all(|p|p[1].0-p[0].1>=6.9));assert!(boxes.last().unwrap().1<=layout.width/2.0-EDGE_PAD+0.01);
            assert!(layout.width<crate::app::WIN_W);
        }}}
    }
    #[test]
    fn clock_12h_keeps_components_separate_in_custom_orders(){
        for order in [DEFAULT_ORDER,[FOCUS,MUSIC,QUOTA,CLOCK,WEATHER,PET,BATTERY,AGENT]] {
            for quotas in 0..=5 {
                let mut layout=Layout::new(true,quotas,true,true,false,true,false,false);
                layout.reorder(&order);layout.widen_clock(40.0);
                let mut boxes:Vec<_>=layout.x.iter().enumerate().filter_map(|(i,x)|x.map(|x|{
                    let width=layout.widths[i];
                    (x-width/2.0,x+width/2.0)
                })).collect();
                boxes.sort_by(|a,b|a.0.total_cmp(&b.0));
                assert!(boxes.windows(2).all(|p|p[1].0-p[0].1>=6.9));
                assert!(boxes.first().unwrap().0>=-layout.width/2.0+EDGE_PAD-0.01);
                assert!(boxes.last().unwrap().1<=layout.width/2.0-EDGE_PAD+0.01);
            }
        }
    }
    #[test]
    fn custom_order_preserves_groups_and_hidden_positions(){
        assert!(!valid_order(&[0,0,2,6,7,9]));
        assert!(!valid_order(&[0,1,2]));
        let order=[FOCUS,MUSIC,QUOTA,CLOCK,WEATHER,PET,BATTERY,AGENT];
        for battery in [false,true]{for count in 0..=5 {
            let mut layout=Layout::new(battery,count,true,true,false,true,false,false);
            let width=layout.width;layout.reorder(&order);
            assert_eq!(layout.width,width);
            let positions:Vec<_>=order.iter().filter_map(|i|layout.x[*i]).collect();
            assert!(positions.windows(2).all(|p|p[0]<p[1]));
            if count>1 {assert_eq!(layout.x[QUOTA+1].unwrap()-layout.x[QUOTA].unwrap(),34.0);}
            let mut boxes:Vec<_>=layout.x.iter().enumerate().filter_map(|(i,x)|x.map(|x|(x-layout.widths[i]/2.0,x+layout.widths[i]/2.0))).collect();
            boxes.sort_by(|a,b|a.0.total_cmp(&b.0));
            assert!(boxes.windows(2).all(|p|p[1].0-p[0].1>=6.9));
        }}
    }
    #[test]
    fn every_component_combination_fits_without_overlaps(){
        for quotas in 0..=8 {for battery in [false,true] {for music in [false,true] {for focus in [false,true] {for bell in [false,true] {for pet in [false,true] {for weather in [false,true] {for agent in [false,true] {
            let layout=Layout::new(battery,quotas,music,focus,bell,pet,weather,agent);
            let mut boxes:Vec<_>=layout.x.iter().enumerate().filter_map(|(i,x)|x.map(|x|(x-layout.widths[i]/2.0,x+layout.widths[i]/2.0))).collect();
            boxes.sort_by(|a,b|a.0.total_cmp(&b.0));
            assert_eq!(layout.quota_slots().count(),quotas);
            if layout.width>96.0 {
                assert!((boxes.first().unwrap().0-(-layout.width/2.0+EDGE_PAD)).abs()<0.01);
                assert!((boxes.last().unwrap().1-(layout.width/2.0-EDGE_PAD)).abs()<0.01);
            }
            assert!(boxes.windows(2).all(|pair|pair[1].0-pair[0].1>=6.9),"组件不能挤在一起：{layout:?}");
            assert_eq!(layout.clearance(CLOCK),1.0);
        }}}}}}}}
    }
    #[test]
    fn weather_defaults_after_the_clock_and_preserves_custom_spacing(){
        for order in [DEFAULT_ORDER,[FOCUS,MUSIC,QUOTA,CLOCK,WEATHER,PET,BATTERY,AGENT],[QUOTA,MUSIC,CLOCK,WEATHER,PET,FOCUS,BATTERY,AGENT]] {
            for quotas in 0..=3 {for battery in [false,true] {
                let mut layout=Layout::new(battery,quotas,true,true,false,true,true,false);
                layout.reorder(&order);
                let (clock,weather)=match (layout.x[CLOCK],layout.x[WEATHER]) {(Some(c),Some(w))=>(c,w),_=>panic!("时钟与天气都应就位：{layout:?}")};
                assert!(weather>clock+22.0+6.9,"天气应紧跟时钟右侧：时钟 {clock}，天气 {weather}");
                // 相邻排列的时钟与天气保持标准组件间距。
                assert!((weather-clock-(38.0+44.0)/2.0-GAP).abs()<0.01);
                let mut boxes:Vec<_>=layout.x.iter().enumerate().filter_map(|(i,x)|x.map(|x|(x-layout.widths[i]/2.0,x+layout.widths[i]/2.0))).collect();
                boxes.sort_by(|a,b|a.0.total_cmp(&b.0));
                assert!(boxes.windows(2).all(|p|p[1].0-p[0].1>=6.9));
                assert!(boxes.last().unwrap().1<=layout.width/2.0-EDGE_PAD+0.01);
                assert!(layout.clearance(WEATHER)>0.99,"天气与额度环不应对撞：{layout:?}");
            }}
        }
    }
    #[test]
    fn weather_chip_widens_symmetrically_without_overlaps(){
        let mut layout=Layout::new(true,3,true,true,false,true,true,false);
        let before=layout.x.clone();
        layout.widen(WEATHER,18.0);
        assert!((layout.widths[WEATHER]-56.0).abs()<0.01);
        for (slot,x) in layout.x.iter().enumerate() {
            match (slot,x) {
                (crate::bar_layout::WEATHER,Some(_))=>{},
                (_,Some(x))=>assert!((*x-before[slot].unwrap()).abs()>8.0,"其余槽位应被推开：{slot}"),
                (_,None)=>{},
            }
        }
        let mut boxes:Vec<_>=layout.x.iter().enumerate().filter_map(|(i,x)|x.map(|x|(x-layout.widths[i]/2.0,x+layout.widths[i]/2.0))).collect();
        boxes.sort_by(|a,b|a.0.total_cmp(&b.0));
        assert!(boxes.windows(2).all(|p|p[1].0-p[0].1>=6.9));
        // 未就位的槽位调用 widen 应是空操作。
        let mut empty=Layout::new(true,0,true,true,false,true,false,false);
        let width=empty.width;empty.widen(WEATHER,20.0);
        assert_eq!(empty.width,width);
    }
    #[test]
    fn replacing_battery_moves_continuously_and_supports_an_instant_state(){
        let before=Layout::new(true,1,true,false,false,true,false,false);let after=Layout::new(false,1,true,false,false,true,false,false);
        let mut motion=Motion::default();motion.step(before.clone(),0.0,true);
        motion.step(after.clone(),1.0/60.0,true);let mid=motion.current().x[QUOTA].unwrap();
        assert!(mid<before.x[QUOTA].unwrap()&&mid>after.x[QUOTA].unwrap());
        for _ in 0..120 {motion.step(after.clone(),1.0/60.0,true);}
        assert!((motion.current().x[QUOTA].unwrap()-after.x[QUOTA].unwrap()).abs()<0.01);
        motion.step(before.clone(),1.0/60.0,false);assert_eq!(motion.current().x,before.x);
    }
    #[test]
    fn music_and_bell_share_the_status_row(){
        let layout=Layout::new(true,0,true,false,true,true,false,false);
        assert!(layout.x[MUSIC].is_some(),"播放音乐时铃铛不能挤掉音乐入口");
        assert!(layout.x[BELL].is_some());
        let mut boxes:Vec<_>=layout.x.iter().enumerate().filter_map(|(i,x)|x.map(|x|(x-layout.widths[i]/2.0,x+layout.widths[i]/2.0))).collect();
        boxes.sort_by(|a,b|a.0.total_cmp(&b.0));
        assert!(boxes.windows(2).all(|pair|pair[1].0-pair[0].1>=6.9));
    }
    #[test]
    fn tray_sits_left_of_the_bell_and_keeps_components_separate(){
        for order in [DEFAULT_ORDER,[FOCUS,MUSIC,QUOTA,CLOCK,WEATHER,PET,BATTERY,AGENT]] {for quotas in 0..=5 {for bell in [false,true] {
            let mut layout=Layout::new(true,quotas,true,true,bell,true,true,true);
            layout.reorder(&order);
            let before=layout.width;
            layout.add_tray(68.0);
            assert_eq!(layout.widths[TRAY],68.0);
            assert!((layout.width-(before+68.0+GAP)).abs()<0.01,"容器应恰好占一槽加一个间距");
            let mut boxes:Vec<_>=layout.x.iter().enumerate().filter_map(|(i,x)|x.map(|x|(x-layout.widths[i]/2.0,x+layout.widths[i]/2.0))).collect();
            boxes.sort_by(|a,b|a.0.total_cmp(&b.0));
            assert!(boxes.windows(2).all(|p|p[1].0-p[0].1>=6.9),"容器插入后组件不能重叠：{layout:?}");
            assert!((boxes.first().unwrap().0-(-layout.width/2.0+EDGE_PAD)).abs()<0.01);
            assert!((boxes.last().unwrap().1-(layout.width/2.0-EDGE_PAD)).abs()<0.01);
            let tray=layout.x[TRAY].unwrap();
            let tray_right=tray+layout.widths[TRAY]/2.0;
            if bell {
                assert!((tray_right-(layout.x[BELL].unwrap()-layout.widths[BELL]/2.0-GAP)).abs()<0.01,"容器应紧贴铃铛左侧");
            } else {
                assert!((tray_right-(layout.width/2.0-EDGE_PAD)).abs()<0.01,"没有铃铛时容器在最右");
            }
        }}}
        // 额度溢出槽不与容器冲突。
        let mut layout=Layout::new(false,8,false,false,false,false,false,false);
        layout.add_tray(40.0);
        assert!(layout.quota_slots().all(|slot|slot!=TRAY));
    }
    #[test]
    fn page_badges_sit_left_of_the_bell_and_keep_components_separate(){
        let items=[(MAIL,30.0),(HEALTH,20.0),(TODO,30.0),(SHELF,20.0),(CALENDAR,20.0),(DND,20.0)];
        for order in [DEFAULT_ORDER,[FOCUS,MUSIC,QUOTA,CLOCK,WEATHER,PET,BATTERY,AGENT]] {for quotas in 0..=5 {for bell in [false,true] {
            let mut layout=Layout::new(true,quotas,true,true,bell,true,true,true);
            layout.reorder(&order);
            let before=layout.width;
            layout.add_extras(&items);
            let content:f32=items.iter().map(|(_,width)|*width).sum::<f32>()+GAP*(items.len()-1) as f32;
            assert!((layout.width-(before+content+GAP)).abs()<0.01,"页面徽标应恰好占内容宽加一个间距");
            let mut boxes:Vec<_>=layout.x.iter().enumerate().filter_map(|(i,x)|x.map(|x|(x-layout.widths[i]/2.0,x+layout.widths[i]/2.0))).collect();
            boxes.sort_by(|a,b|a.0.total_cmp(&b.0));
            assert!(boxes.windows(2).all(|pair|pair[1].0-pair[0].1>=6.9),"徽标插入后组件不能重叠：{layout:?}");
            assert!((boxes.first().unwrap().0-(-layout.width/2.0+EDGE_PAD)).abs()<0.01);
            assert!((boxes.last().unwrap().1-(layout.width/2.0-EDGE_PAD)).abs()<0.01);
            for pair in items.windows(2) {
                let right=layout.x[pair[0].0].unwrap()+pair[0].1/2.0;
                let left=layout.x[pair[1].0].unwrap()-pair[1].1/2.0;
                assert!((left-right-GAP).abs()<0.01,"徽标按给定顺序相邻排列");
            }
            let last=items[items.len()-1];
            let right=layout.x[last.0].unwrap()+last.1/2.0;
            if bell {
                assert!((right-(layout.x[BELL].unwrap()-layout.widths[BELL]/2.0-GAP)).abs()<0.01,"徽标应紧贴铃铛左侧");
            } else {
                assert!((right-(layout.width/2.0-EDGE_PAD)).abs()<0.01,"没有铃铛时徽标在最右");
            }
        }}}
        // 额度溢出槽不与页面徽标冲突；空列表是空操作。
        let mut layout=Layout::new(false,8,false,false,false,false,false,false);
        layout.add_extras(&[(MAIL,30.0)]);
        assert!(layout.quota_slots().all(|slot|slot!=MAIL));
        let width=layout.width;layout.add_extras(&[]);
        assert_eq!(layout.width,width);
    }
    #[test]
    fn quota_mini_layout_recenters_on_its_content(){
        let mut mini=Layout::new(false,2,false,false,false,false,false,false);
        mini.hide_clock();
        let width=mini.center_content();
        assert!(width>0.0);
        let left=mini.x.iter().enumerate().filter_map(|(i,x)|x.map(|x|x-mini.widths[i]/2.0)).fold(f32::INFINITY,f32::min);
        let right=mini.x.iter().enumerate().filter_map(|(i,x)|x.map(|x|x+mini.widths[i]/2.0)).fold(f32::NEG_INFINITY,f32::max);
        assert!((left+right).abs()<0.01,"迷你布局应按内容居中");
        assert!((width-(right-left)).abs()<0.01);
    }
}
