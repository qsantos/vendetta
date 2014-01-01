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

#include "../graphics.h"

struct subwindow
{
	char visible;
	float x;
	float y;
	const wchar_t* name;

	sfView* view;
};

#define SW_WIDTH  307.f
#define SW_HEIGHT 356.f

void subwindow_init(subwindow_t* w, graphics_t* g, const wchar_t* name, float x, float y);
void subwindow_exit(subwindow_t* w);

char subwindow_cursor(subwindow_t* w, int x, int y);
char subwindow_draw  (subwindow_t* w, graphics_t* g);
char subwindow_catch (subwindow_t* w, int x, int y, int t);
char subwindow_wheel (subwindow_t* w, int x, int y, int delta);

#endif
