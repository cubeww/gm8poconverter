/// ONLINE
// World: The name of the world object
if(World.__ONLINE_vis <= 1){
if(sprite_exists(sprite_index)){
draw_sprite_ext(sprite_index, image_index, x, y, image_xscale, image_yscale, image_angle, c_white, image_alpha);
if(World.__ONLINE_vis == 0){
__ONLINE__alpha = draw_get_alpha();
__ONLINE__color = draw_get_color();
draw_set_alpha(image_alpha);
draw_set_font(__ONLINE_ftOnlinePlayerName);
draw_set_valign(fa_center);
draw_set_halign(fa_center);
draw_set_color(c_black);
__ONLINE_border = 2;
__ONLINE_padding = 30;
__ONLINE_xx = x;
__ONLINE_yy = y-__ONLINE_padding;
draw_set_alpha(1);
draw_text(__ONLINE_xx+__ONLINE_border, __ONLINE_yy, __ONLINE_name);
draw_text(__ONLINE_xx, __ONLINE_yy+__ONLINE_border, __ONLINE_name);
draw_text(__ONLINE_xx-__ONLINE_border, __ONLINE_yy, __ONLINE_name);
draw_text(__ONLINE_xx, __ONLINE_yy-__ONLINE_border, __ONLINE_name);
draw_set_color(c_white);
draw_text(__ONLINE_xx, __ONLINE_yy, __ONLINE_name);
draw_set_alpha(__ONLINE__alpha);
draw_set_color(__ONLINE__color);
if(font_exists(0)){
draw_set_font(0);
}
draw_set_valign(fa_top);
draw_set_halign(fa_left);
}
}
}