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

#ifndef O_SUBWINDOW_H
#define O_SUBWINDOW_H

typedef struct subwindow subwindow_t;

#include <SFML/Graphics.h>

struct subwindow
{
	char visible;
	float x;
	float y;
	char* name;

	float height;
	float scrolling;
	sfView* view;
};

#define SW_WIDTH  307.f
#define SW_HEIGHT 356.f

#include "../graphics.h"
#include "../assets.h"

void subwindow_init(subwindow_t* w, graphics_t* g, const char* name, float x, float y);
void subwindow_exit(subwindow_t* w);

char subwindow_cursor(subwindow_t* w, graphics_t* g);
char subwindow_draw  (subwindow_t* w, graphics_t* g, assets_t* a);
char subwindow_catch (subwindow_t* w, graphics_t* g, int t);
char subwindow_wheel (subwindow_t* w, graphics_t* g, int delta);

sfVector2f subwindow_mouse(subwindow_t* w, graphics_t* g);

#endif
