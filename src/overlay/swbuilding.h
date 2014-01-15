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

#ifndef O_SWBUILDING_H
#define O_SWBUILDING_H

typedef struct swbuilding swbuilding_t;

#include "subwindow.h"

struct swbuilding
{
	subwindow_t w;
};

#include "../game.h"

void swbuilding_init(swbuilding_t* w, graphics_t* g);
void swbuilding_exit(swbuilding_t* w);

int  swbuilding_draw  (swbuilding_t* w, game_t* g, char do_draw);
int  swbuilding_cursor(swbuilding_t* w, game_t* g, int x, int y);
char swbuilding_catch (swbuilding_t* w, game_t* g, int x, int y, int t);

#endif
