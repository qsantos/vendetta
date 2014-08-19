/*\
 *  Role playing, management and strategy game
 *  Copyright (C) 2013-2014 Quentin SANTOS
 *
 *  This program is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  This program is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with this program.  If not, see <http://www.gnu.org/licenses/>.
\*/

#ifndef WIDGET_H
#define WIDGET_H

#include "graphics.h"
#include "assets.h"

void draw_cursor   (graphics_t* gr, assets_t* a, int t);
void draw_roundrect(graphics_t* gr, float x, float y, float w, float h);
void draw_tooltip  (graphics_t* gr, assets_t* a, const char* txt);

// passive widgets

// p is the progress
// c ==  1 sets the outline to orange
//      -2 sets the inner color to white (work progress)
//      -2 sets the inner color to faint white (used for sliders)
//      -3                         red (attack)
//      -4                         blue (defense)
void draw_progressbar(graphics_t* gr, float x, float y, float w, float h, float p, char c);

// h is the total height of the scroll bar
// r is the proportion the marker should occupy
// p is the vertical progress
void draw_scrollbar  (graphics_t* gr, float x, float y, float w, float h, float r, float p);

// clickable widgets
char draw_button(graphics_t* gr, assets_t* a, float x, float y, const char* name, char enabled, char do_draw);
char draw_toggle(graphics_t* gr, assets_t* a, float x, float y, const char* name, char* v, char do_draw);
void draw_slider(graphics_t* gr, assets_t* a, float x, float y, const char* name, int*  v, int min, int max, char do_draw);

// input (two functions)
void draw_input(graphics_t* gr, assets_t* a, float x, float y, const char* name, const char* value, char do_draw);
void input_type(char* value, size_t n, sfUint32  c);

#endif
