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

#include "subwindow.h"

#include "../util.h"

void subwindow_init(subwindow_t* w, graphics_t* g, const char* name, float x, float y)
{
	w->name = strdup(name);
	w->x = x;
	w->y = y;
	w->visible = 1;
	w->view = sfView_copy(sfRenderWindow_getDefaultView(g->render));
	sfView_reset(w->view, (sfFloatRect){0,0,SW_WIDTH-40,SW_HEIGHT-70});
}

void subwindow_exit(subwindow_t* w)
{
	free(w->name);
}

char subwindow_cursor(subwindow_t* w, int x, int y)
{
	if (!w->visible)
		return 0;

	sfFloatRect rect = {w->x, w->y, SW_WIDTH, SW_HEIGHT};
	return sfFloatRect_contains(&rect, x, y);
}

char subwindow_draw(subwindow_t* w, graphics_t* g)
{
	if (!w->visible)
		return 0;

	sfVector2f pos = {w->x, w->y};

	static sfSprite* sprite = NULL;
	if (sprite == NULL)
	{
		int id = graphics_spriteForImg(g, "subwindow.png");
		sprite = g->sprites[id];
	}
	sfSprite_setPosition(sprite, pos);
	sfRenderWindow_drawSprite(g->render, sprite, NULL);

	static sfText* text = NULL;
	if (text == NULL)
	{
		text = sfText_create();
		sfColor col = {255, 255, 255, 255};
		sfText_setColor        (text, col);
		sfText_setFont         (text, g->font);
		sfText_setCharacterSize(text, 18);
	}
	sfText_setUTF8(text, w->name);
	sfFloatRect rect = sfText_getLocalBounds(text);
	pos.x += (SW_WIDTH-rect.width)/2;
	pos.y += + 20;
	sfText_setPosition(text, pos);

	sfRenderWindow_drawText(g->render, text, NULL);

	sfVector2u size = sfRenderWindow_getSize(g->render);
	sfFloatRect viewport = {(w->x+20)/size.x, (w->y+50)/size.y, (SW_WIDTH-40)/size.x, (SW_HEIGHT-70)/size.y};
	sfView_setViewport(w->view, viewport);
	sfRenderWindow_setView(g->render, w->view);

	return 1;
}

char subwindow_catch(subwindow_t* w, int x, int y, int t)
{
	(void) t;

	if (!w->visible)
		return 0;

	sfFloatRect rect = {w->x, w->y, SW_WIDTH, SW_HEIGHT};
	return sfFloatRect_contains(&rect, x, y);
}

char subwindow_wheel(subwindow_t* w, int x, int y, int delta)
{
	sfFloatRect rect = {w->x, w->y, SW_WIDTH, SW_HEIGHT};
	if (!sfFloatRect_contains(&rect, x, y))
		return 0;

	sfView_move(w->view, (sfVector2f){0,-10*delta});
	return 1;
}
