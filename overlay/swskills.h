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

#ifndef O_SWSKILLS_H
#define O_SWSKILLS_H

typedef struct swskills swskills_t;

#include "subwindow.h"

struct swskills
{
	subwindow_t w;
};

#include "../game.h"

void swskills_init(swskills_t* w, graphics_t* g);
void swskills_exit(swskills_t* w);

char swskills_cursor(swskills_t* w, game_t* g, int x, int y);
void swskills_draw  (swskills_t* w, game_t* g);
char swskills_catch (swskills_t* w, game_t* g, int x, int y, int t);

#endif
