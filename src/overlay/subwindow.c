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

#include <stdlib.h>
#include <math.h>

#include "../string.h"
#include "../widgets.h"

void subwindow_init(subwindow_t* w, graphics_t* g, const char* name, float x, float y)
{
	w->name = strdup(name);
	w->x = x;
	w->y = y;
	w->visible = 0;

	w->height = 0;
	w->scrolling = 0;
	w->view = sfView_copy(sfRenderWindow_getDefaultView(g->render));
	sfView_reset(w->view, (sfFloatRect){0,0,SW_WIDTH-40,SW_HEIGHT-70});
}

void subwindow_exit(subwindow_t* w)
{
	sfView_destroy(w->view);
	free(w->name);
}

char subwindow_cursor(subwindow_t* w, graphics_t* g)
{
	if (!w->visible)
		return 0;

	sfVector2i mouse = sfMouse_getPosition((sfWindow*) g->render);
	sfFloatRect rect = {w->x, w->y, SW_WIDTH, SW_HEIGHT};
	return sfFloatRect_contains(&rect, mouse.x, mouse.y);
}

char subwindow_draw(subwindow_t* w, graphics_t* g)
{
	if (!w->visible)
		return 0;

	sfVector2f pos = {w->x, w->y};

	static sfSprite* sprite = NULL;
	if (sprite == NULL)
		sprite = graphics_sprite(g, "data/subwindow.png");
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

	float min_scrolling = 0;
	float max_scrolling = fmax(0, w->height - (SW_HEIGHT-70));
	if (w->scrolling > max_scrolling)
	{
		float delta = max_scrolling - w->scrolling;
		w->scrolling += delta;
		sfView_move(w->view, (sfVector2f){0,delta});
	}
	if (max_scrolling > min_scrolling)
	{
		float p = w->scrolling / (max_scrolling - min_scrolling);
		float r = (SW_HEIGHT - 40) / w->height;
		draw_scrollbar(g, w->x+SW_WIDTH-20, w->y+50, 7, SW_HEIGHT-70, r, p);
	}

	sfVector2u size = sfRenderWindow_getSize(g->render);
	sfFloatRect viewport = {(w->x+20)/size.x, (w->y+50)/size.y, (SW_WIDTH-40)/size.x, (SW_HEIGHT-70)/size.y};
	sfView_setViewport(w->view, viewport);
	sfRenderWindow_setView(g->render, w->view);

	return 1;
}

char subwindow_catch(subwindow_t* w, graphics_t* g, int t)
{
	(void) t;

	if (!subwindow_cursor(w, g))
		return 0;

	return 1;
}

char subwindow_wheel(subwindow_t* w, graphics_t* g, int delta)
{
	if (!subwindow_cursor(w, g))
		return 0;

	float min_scrolling = 0;
	float max_scrolling = w->height - (SW_HEIGHT-70);

	if (max_scrolling > min_scrolling)
	{
		delta *= -20;
		delta = fmax(delta, min_scrolling-w->scrolling);
		delta = fmin(delta, max_scrolling-w->scrolling);
		w->scrolling += delta;
		sfView_move(w->view, (sfVector2f){0,delta});
	}
	return 1;
}

sfVector2f subwindow_mouse(subwindow_t* w, graphics_t* g)
{
	sfVector2i mouse = sfMouse_getPosition((sfWindow*) g->render);
	return sfRenderWindow_mapPixelToCoords(g->render, mouse, w->view);
}
