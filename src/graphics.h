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

#ifndef GRAPHICS_H
#define GRAPHICS_H

#include <SFML/Graphics.h>

typedef struct graphics graphics_t;

struct graphics
{
	sfRenderWindow* render;

	sfView* world_view;
	sfView* overlay_view;
};

void graphics_init(graphics_t* g);
void graphics_exit(graphics_t* g);

void sfText_setUTF8(sfText* text, const char* string);

// additional membership tests
char sfText_contains  (sfText*   text,   sfVector2f point);
char sfSprite_contains(sfSprite* sprite, sfVector2f point);

#endif
